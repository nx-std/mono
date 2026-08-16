//! SSL (`ssl` / `ssl:s`) service implementation.
//!
//! Provides client-mode TLS connectivity via the Nintendo Switch SSL
//! service. The service operates in domain mode with a session pool for
//! concurrent IPC dispatch, matching libnx's `SessionMgr` pattern.
//!
//! ## Architecture
//!
//! [`SslService`] is the root service wrapper. From it, callers create
//! [`SslContext`] sub-objects, which in turn create [`SslConnection`]
//! sub-objects for individual TLS sessions.
//!
//! ## Divergence from libnx
//!
//! libnx's `ssl.c` keeps a guarded global singleton with a
//! `SslServiceType` enum, calls `SetInterfaceVersion` during init, and
//! uses `SessionMgr` for concurrent dispatch. This crate follows the
//! same domain + session-pool pattern but exposes connect/context/
//! connection as composable types without global state.
//!
//! Per IC-4, this crate is hosversion-unaware. Hosversion-gated commands
//! (e.g., `FlushSessionCache` 5.0.0+, `SetDebugOption` 6.0.0+,
//! `ClearTls12FallbackFlag` 14.0.0+, system-only cmds 15.0.0+, DTLS
//! cmds 16.0.0+) are exposed unconditionally: callers choose based on
//! the target firmware version. The `SetInterfaceVersion` internal
//! command is exposed as a public method so callers can issue it after
//! connect if needed.

#![no_std]

extern crate alloc;
extern crate nx_panic_handler as _; // provides #[panic_handler]

use alloc::vec::Vec;

use nx_service_bsd::RawSockAddr;
use nx_service_sm::SmService;
use nx_sf::service::{
    ConvertToDomainError,
    DispatchError,
    Domain,
    DomainObject,
    Session,
    clone_current_object,
};

mod cmif;
mod dispatch;
#[cfg(feature = "ffi")]
pub mod ffi;
mod proto;
mod session;
pub mod types;

pub use nx_sf::service::DispatchError as IpcDispatchError;

use self::session::SessionPool;
pub use self::{
    cmif::{
        CreateConnectionError,
        CreateContextError,
        GenerateKeyAndCertError,
        RemovePkiError,
    },
    proto::{
        SERVICE_NAME,
        SERVICE_NAME_SYSTEM,
    },
    types::{
        AlpnProtoState,
        CaCertificateId,
        CertificateFormat,
        CipherInfo,
        ContextOption,
        DebugOptionType,
        FlushSessionCacheOptionType,
        InternalPki,
        IoMode,
        KeyAndCertParams,
        NoDescriptor,
        OptionType,
        PollEvent,
        PrivateOptionType,
        RenegotiationMode,
        ServerCertDetailEntry,
        ServerCertDetailHeader,
        SessionCacheMode,
        SocketFd,
        SslServiceType,
        SslVersion,
        TrustedCertStatus,
        UnknownOption,
        VerifyOption,
    },
};

/// Connected SSL service wrapper.
///
/// Operates in domain mode with a session pool sized by the [`SessionCount`] passed to
/// [`connect_cmif`]. Dropping the service closes all pool sessions.
pub struct SslService {
    pool: SessionPool,
    system: bool,
}

// SAFETY: every field is either an immutable kernel handle wrapper or a
// `nx_std_sync::Mutex` / `Condvar` based pool. Concurrent IPC calls from
// different threads acquire distinct pool slots.
unsafe impl Send for SslService {}
unsafe impl Sync for SslService {}

impl SslService {
    /// Creates an SSL context.
    ///
    /// `ssl_version` is a bitmask of [`SslVersion`] flags.
    pub fn create_context(
        &self,
        ssl_version: SslVersion,
    ) -> Result<SslContext<'_>, CreateContextError> {
        let guard = self.pool.acquire();
        let raw_object_id = cmif::create_context(guard.domain(), ssl_version, self.system)?;
        // SAFETY: `raw_object_id` was just returned by `cmif::create_context`
        // on this same domain; no other `DomainObject` references it.
        // All pool domains share the same server-side object table (see
        // `connect_cmif`), so the id remains valid for the lifetime of any
        // pool slot.
        let object = DomainObject::from_raw_unchecked(guard.domain(), raw_object_id)
            .ok_or(CreateContextError::MissingObject)?;
        Ok(SslContext {
            object,
            pool: &self.pool,
        })
    }

    /// Gets the total context count.
    pub fn get_context_count(&self) -> Result<u32, DispatchError> {
        let guard = self.pool.acquire();
        cmif::get_context_count(guard.domain())
    }

    /// Gets built-in certificates (pre-3.0.0 variant, no output count).
    pub fn get_certificates_legacy(
        &self,
        buffer: &mut [u8],
        ca_cert_ids: &[u32],
    ) -> Result<(), DispatchError> {
        let guard = self.pool.acquire();
        cmif::get_certificates_legacy(guard.domain(), buffer, ca_cert_ids)
    }

    /// Gets built-in certificates (3.0.0+, returns output count).
    pub fn get_certificates(
        &self,
        buffer: &mut [u8],
        ca_cert_ids: &[u32],
    ) -> Result<u32, DispatchError> {
        let guard = self.pool.acquire();
        cmif::get_certificates(guard.domain(), buffer, ca_cert_ids)
    }

    /// Gets the required buffer size for the given certificate IDs.
    pub fn get_certificate_buf_size(&self, ca_cert_ids: &[u32]) -> Result<u32, DispatchError> {
        let guard = self.pool.acquire();
        cmif::get_certificate_buf_size(guard.domain(), ca_cert_ids)
    }

    /// Sets the interface version (3.0.0+ internal command).
    ///
    /// libnx calls this automatically during init with version values:
    /// - 0x1 for 3.0.0+
    /// - 0x2 for 5.0.0+
    /// - 0x3 for 6.0.0+
    pub fn set_interface_version(&self, version: u32) -> Result<(), DispatchError> {
        let guard = self.pool.acquire();
        cmif::set_interface_version(guard.domain(), version)
    }

    /// Flushes the session cache (5.0.0+).
    pub fn flush_session_cache(
        &self,
        hostname: &[u8],
        option_type: FlushSessionCacheOptionType,
    ) -> Result<u32, DispatchError> {
        let guard = self.pool.acquire();
        cmif::svc_flush_session_cache(guard.domain(), hostname, option_type)
    }

    /// Sets a debug option (6.0.0+).
    pub fn set_debug_option(
        &self,
        debug_type: DebugOptionType,
        buffer: &[u8],
    ) -> Result<(), DispatchError> {
        let guard = self.pool.acquire();
        cmif::set_debug_option(guard.domain(), debug_type, buffer)
    }

    /// Gets a debug option (6.0.0+).
    pub fn get_debug_option(
        &self,
        debug_type: DebugOptionType,
        buffer: &mut [u8],
    ) -> Result<(), DispatchError> {
        let guard = self.pool.acquire();
        cmif::get_debug_option(guard.domain(), debug_type, buffer)
    }

    /// Clears the TLS 1.2 fallback flag (14.0.0+).
    pub fn clear_tls12_fallback_flag(&self) -> Result<(), DispatchError> {
        let guard = self.pool.acquire();
        cmif::clear_tls12_fallback_flag(guard.domain())
    }

    /// Sets the thread core mask (15.0.0+, system only).
    pub fn set_thread_core_mask(&self, mask: u64) -> Result<(), DispatchError> {
        let guard = self.pool.acquire();
        cmif::set_thread_core_mask(guard.domain(), mask)
    }

    /// Gets the thread core mask (15.0.0+, system only).
    pub fn get_thread_core_mask(&self) -> Result<u64, DispatchError> {
        let guard = self.pool.acquire();
        cmif::get_thread_core_mask(guard.domain())
    }
}

/// SSL context sub-object obtained via [`SslService::create_context`].
///
/// The lifetime parameter ties the context to its parent service so the
/// underlying domain session outlives the sub-object. Dropping the
/// context sends a per-object close request on the domain.
pub struct SslContext<'svc> {
    object: DomainObject<'svc>,
    pool: &'svc SessionPool,
}

impl SslContext<'_> {
    /// Sets a context option.
    pub fn set_option(&self, option: ContextOption, value: i32) -> Result<(), DispatchError> {
        cmif::ctx_set_option(self.object.as_borrowed(), option, value)
    }

    /// Gets a context option.
    pub fn get_option(&self, option: ContextOption) -> Result<i32, DispatchError> {
        cmif::ctx_get_option(self.object.as_borrowed(), option)
    }

    /// Creates a connection sub-object of the requested kind.
    pub fn create_connection(
        &self,
        kind: ConnectionKind,
    ) -> Result<SslConnection<'_>, CreateConnectionError> {
        let raw_object_id = cmif::create_connection(self.object.as_borrowed(), kind)?;
        let guard = self.pool.acquire();
        // SAFETY: `raw_object_id` was just returned by `cmif::create_connection`
        // on a pool domain; all pool domains share the same server-side
        // object table (see `connect_cmif`), and no other `DomainObject`
        // references this id.
        let object = DomainObject::from_raw_unchecked(guard.domain(), raw_object_id)
            .ok_or(CreateConnectionError::MissingObject)?;
        Ok(SslConnection { object })
    }

    /// Gets the connection count for this context.
    pub fn get_connection_count(&self) -> Result<u32, DispatchError> {
        cmif::get_connection_count(self.object.as_borrowed())
    }

    /// Imports server PKI certificate(s). Returns the assigned ID.
    pub fn import_server_pki(
        &self,
        cert_data: &[u8],
        format: CertificateFormat,
    ) -> Result<u64, DispatchError> {
        cmif::import_server_pki(self.object.as_borrowed(), cert_data, format)
    }

    /// Imports a client PKI (PKCS#12). Returns the assigned ID.
    pub fn import_client_pki(&self, pkcs12: &[u8], password: &[u8]) -> Result<u64, DispatchError> {
        cmif::import_client_pki(self.object.as_borrowed(), pkcs12, password)
    }

    /// Removes a PKI or CRL by ID.
    ///
    /// Tries RemoveServerPki, RemoveClientPki, and (if `include_crl` is
    /// true) RemoveCrl in order, matching libnx's behavior.
    pub fn remove_pki(&self, id: u64, include_crl: bool) -> Result<(), RemovePkiError> {
        cmif::remove_pki(self.object.as_borrowed(), id, include_crl)
    }

    /// Registers an internal PKI. Returns the assigned ID.
    pub fn register_internal_pki(&self, pki: InternalPki) -> Result<u64, DispatchError> {
        cmif::register_internal_pki(self.object.as_borrowed(), pki)
    }

    /// Adds a policy OID string.
    pub fn add_policy_oid(&self, oid: &[u8]) -> Result<(), DispatchError> {
        cmif::add_policy_oid(self.object.as_borrowed(), oid)
    }

    /// Imports a CRL (3.0.0+). Returns the assigned ID.
    pub fn import_crl(&self, crl_data: &[u8]) -> Result<u64, DispatchError> {
        cmif::import_crl(self.object.as_borrowed(), crl_data)
    }

    /// Imports client cert and key PKI (16.0.0+). Returns the assigned ID.
    pub fn import_client_cert_key_pki(
        &self,
        cert: &[u8],
        key: &[u8],
        format: CertificateFormat,
    ) -> Result<u64, DispatchError> {
        cmif::import_client_cert_key_pki(self.object.as_borrowed(), cert, key, format)
    }

    /// Generates a private key and certificate (16.0.0+).
    ///
    /// Returns `(cert_size, key_size)`: the actual sizes written to the
    /// output buffers.
    pub fn generate_private_key_and_cert(
        &self,
        cert_buf: &mut [u8],
        key_buf: &mut [u8],
        params: &KeyAndCertParams,
    ) -> Result<(u32, u32), GenerateKeyAndCertError> {
        let out = cmif::generate_private_key_and_cert(
            self.object.as_borrowed(),
            cert_buf,
            key_buf,
            1,
            params,
        )?;
        Ok((out.cert_size, out.key_size))
    }
}

/// Which of the two connection-creating commands to send.
///
/// The service answers `CreateConnection` and `CreateConnectionForSystem` at different request
/// ids, and a caller may ask for either regardless of which service variant it connected to. This
/// is that choice, named rather than spelled as a second function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    /// The connection an application uses.
    Application,
    /// The connection a system title uses (15.0.0+).
    System,
}

/// SSL connection sub-object obtained via [`SslContext::create_connection`].
///
/// The lifetime parameter ties the connection to its parent context's
/// domain session. Dropping the connection sends a per-object close
/// request on the domain.
pub struct SslConnection<'ctx> {
    object: DomainObject<'ctx>,
}

impl SslConnection<'_> {
    /// Sets the socket descriptor. Returns the one the connection gave up, if it held one.
    pub fn set_socket_descriptor(
        &self,
        sockfd: impl Into<SocketFd>,
    ) -> Result<Option<SocketFd>, DispatchError> {
        cmif::set_socket_descriptor(self.object.as_borrowed(), sockfd)
    }

    /// Sets the host name for TLS verification.
    pub fn set_host_name(&self, name: &[u8]) -> Result<(), DispatchError> {
        cmif::set_host_name(self.object.as_borrowed(), name)
    }

    /// Sets the verify option bitmask.
    pub fn set_verify_option(&self, options: VerifyOption) -> Result<(), DispatchError> {
        cmif::set_verify_option(self.object.as_borrowed(), options)
    }

    /// Sets the I/O mode.
    pub fn set_io_mode(&self, mode: IoMode) -> Result<(), DispatchError> {
        cmif::set_io_mode(self.object.as_borrowed(), mode)
    }

    /// Gets the socket descriptor the connection holds, if it holds one.
    pub fn get_socket_descriptor(&self) -> Result<Option<SocketFd>, DispatchError> {
        cmif::get_socket_descriptor(self.object.as_borrowed())
    }

    /// Gets the host name string. Returns the string length.
    pub fn get_host_name(&self, buffer: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::get_host_name(self.object.as_borrowed(), buffer)
    }

    /// Gets the verify option bitmask.
    pub fn get_verify_option(&self) -> Result<VerifyOption, DispatchError> {
        cmif::get_verify_option(self.object.as_borrowed()).map(VerifyOption::from_bits_retain)
    }

    /// Gets the I/O mode.
    pub fn get_io_mode(&self) -> Result<u32, DispatchError> {
        cmif::get_io_mode(self.object.as_borrowed())
    }

    /// Performs a TLS handshake without requesting server cert.
    pub fn do_handshake(&self) -> Result<(), DispatchError> {
        cmif::do_handshake(self.object.as_borrowed())
    }

    /// Performs a TLS handshake and retrieves server cert data.
    ///
    /// Returns `(data_size, total_certs)`.
    pub fn do_handshake_get_server_cert(
        &self,
        server_certbuf: &mut [u8],
    ) -> Result<(u32, u32), DispatchError> {
        let out = cmif::do_handshake_get_server_cert(self.object.as_borrowed(), server_certbuf)?;
        Ok((out.data_size, out.total_certs))
    }

    /// Reads data from the TLS connection. Returns bytes transferred.
    pub fn read(&self, buffer: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::read(self.object.as_borrowed(), buffer)
    }

    /// Writes data to the TLS connection. Returns bytes transferred.
    pub fn write(&self, buffer: &[u8]) -> Result<u32, DispatchError> {
        cmif::write(self.object.as_borrowed(), buffer)
    }

    /// Gets the number of pending bytes.
    pub fn pending(&self) -> Result<i32, DispatchError> {
        cmif::pending(self.object.as_borrowed())
    }

    /// Peeks at data without consuming it. Returns bytes read.
    pub fn peek(&self, buffer: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::peek(self.object.as_borrowed(), buffer)
    }

    /// Polls the connection for events.
    pub fn poll(&self, in_pollevent: PollEvent, timeout: u32) -> Result<PollEvent, DispatchError> {
        cmif::poll(self.object.as_borrowed(), in_pollevent, timeout)
            .map(PollEvent::from_bits_retain)
    }

    /// Gets the verify cert error (clears the stored value).
    pub fn get_verify_cert_error(&self) -> Result<(), DispatchError> {
        cmif::get_verify_cert_error(self.object.as_borrowed())
    }

    /// Gets the needed server cert buffer size.
    pub fn get_needed_server_cert_buffer_size(&self) -> Result<u32, DispatchError> {
        cmif::get_needed_server_cert_buffer_size(self.object.as_borrowed())
    }

    /// Sets the session cache mode.
    pub fn set_session_cache_mode(&self, mode: SessionCacheMode) -> Result<(), DispatchError> {
        cmif::set_session_cache_mode(self.object.as_borrowed(), mode)
    }

    /// Gets the session cache mode.
    pub fn get_session_cache_mode(&self) -> Result<u32, DispatchError> {
        cmif::get_session_cache_mode(self.object.as_borrowed())
    }

    /// Flushes the connection's session cache.
    pub fn flush_session_cache(&self) -> Result<(), DispatchError> {
        cmif::flush_session_cache(self.object.as_borrowed())
    }

    /// Sets the renegotiation mode.
    pub fn set_renegotiation_mode(&self, mode: RenegotiationMode) -> Result<(), DispatchError> {
        cmif::set_renegotiation_mode(self.object.as_borrowed(), mode)
    }

    /// Gets the renegotiation mode.
    pub fn get_renegotiation_mode(&self) -> Result<u32, DispatchError> {
        cmif::get_renegotiation_mode(self.object.as_borrowed())
    }

    /// Sets a connection option.
    pub fn set_option(&self, option: OptionType, flag: bool) -> Result<(), DispatchError> {
        cmif::set_option(self.object.as_borrowed(), option, flag)
    }

    /// Gets a connection option.
    pub fn get_option(&self, option: OptionType) -> Result<bool, DispatchError> {
        cmif::get_option(self.object.as_borrowed(), option)
    }

    /// Gets verify cert errors into a buffer.
    ///
    /// Returns `(count_0, count_1)`. The two are expected to agree; this
    /// crate returns both and leaves the comparison to the caller.
    pub fn get_verify_cert_errors(&self, errors: &mut [u32]) -> Result<(u32, u32), DispatchError> {
        cmif::get_verify_cert_errors(self.object.as_borrowed(), errors)
    }

    /// Gets cipher info (4.0.0+).
    pub fn get_cipher_info(&self, out: &mut CipherInfo) -> Result<(), DispatchError> {
        cmif::get_cipher_info(self.object.as_borrowed(), out)
    }

    /// Sets the next ALPN protocol list (9.0.0+).
    pub fn set_next_alpn_proto(&self, proto_list: &[u8]) -> Result<(), DispatchError> {
        cmif::set_next_alpn_proto(self.object.as_borrowed(), proto_list)
    }

    /// Gets the next ALPN protocol (9.0.0+).
    ///
    /// Returns `(state, proto_size)`. The protocol string is written to
    /// `buffer`.
    pub fn get_next_alpn_proto(
        &self,
        buffer: &mut [u8],
    ) -> Result<(AlpnProtoState, u32), DispatchError> {
        let out = cmif::get_next_alpn_proto(self.object.as_borrowed(), buffer)?;
        Ok((AlpnProtoState::from_raw(out.state), out.proto_size))
    }

    /// Sets DTLS socket descriptor (16.0.0+). Returns the previous sockfd.
    pub fn set_dtls_socket_descriptor(
        &self,
        sockfd: impl Into<SocketFd>,
        sockaddr: &RawSockAddr,
    ) -> Result<Option<SocketFd>, DispatchError> {
        cmif::set_dtls_socket_descriptor(self.object.as_borrowed(), sockfd, sockaddr)
    }

    /// Gets DTLS handshake timeout in nanoseconds (16.0.0+).
    pub fn get_dtls_handshake_timeout(&self) -> Result<u64, DispatchError> {
        cmif::get_dtls_handshake_timeout(self.object.as_borrowed())
    }

    /// Sets a private option (pre-17.0.0, bool+option layout).
    pub fn set_private_option_legacy(
        &self,
        option: PrivateOptionType,
        value: bool,
    ) -> Result<(), DispatchError> {
        cmif::set_private_option_legacy(self.object.as_borrowed(), option, value)
    }

    /// Sets a private option (17.0.0+, option+value layout).
    pub fn set_private_option(
        &self,
        option: PrivateOptionType,
        value: u32,
    ) -> Result<(), DispatchError> {
        cmif::set_private_option(self.object.as_borrowed(), option, value)
    }

    /// Sets SRTP ciphers (16.0.0+).
    pub fn set_srtp_ciphers(&self, ciphers: &[u16]) -> Result<(), DispatchError> {
        cmif::set_srtp_ciphers(self.object.as_borrowed(), ciphers)
    }

    /// Gets the negotiated SRTP cipher (16.0.0+).
    pub fn get_srtp_cipher(&self) -> Result<u16, DispatchError> {
        cmif::get_srtp_cipher(self.object.as_borrowed())
    }

    /// Exports keying material (16.0.0+).
    pub fn export_keying_material(
        &self,
        outbuf: &mut [u8],
        label: &[u8],
        context: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::export_keying_material(self.object.as_borrowed(), outbuf, label, context)
    }

    /// Sets I/O timeout (16.0.0+).
    pub fn set_io_timeout(&self, timeout: u32) -> Result<(), DispatchError> {
        cmif::set_io_timeout(self.object.as_borrowed(), timeout)
    }

    /// Gets I/O timeout (16.0.0+).
    pub fn get_io_timeout(&self) -> Result<u32, DispatchError> {
        cmif::get_io_timeout(self.object.as_borrowed())
    }
}

/// Connects to the SSL service using CMIF.
///
/// Sets up domain conversion and a session pool of `sessions` slots. The `system` parameter
/// selects `ssl:s` (15.0.0+) over `ssl`; nothing here checks the firmware, so a caller asking for
/// the system variant on firmware without it gets the service manager's answer.
pub fn connect_cmif(
    sm: &SmService,
    system: bool,
    sessions: SessionCount,
) -> Result<SslService, ConnectCmifError> {
    let service_name = if system {
        proto::SERVICE_NAME_SYSTEM
    } else {
        proto::SERVICE_NAME
    };

    let handle = sm
        .get_service_handle_cmif(service_name)
        .map_err(ConnectCmifError::GetService)?;

    let session = Session::open(handle);
    let pointer_buffer_size = session.pointer_buffer_size();

    let creator = session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectCmifError::ConvertToDomain(err))?;

    let slots = sessions.to_len();
    let mut sessions: Vec<Domain> = Vec::with_capacity(slots);
    sessions.push(creator);
    for _ in 1..slots {
        let cloned_handle =
            clone_current_object(sessions[0].handle()).map_err(ConnectCmifError::CloneSession)?;
        // SAFETY: cloning a domain session yields another kernel handle that
        // addresses the same domain object table on the server side, so the
        // original interface keeps the id the conversion assigned it.
        let cloned_domain =
            Domain::new_unchecked(cloned_handle, pointer_buffer_size, sessions[0].object_id());
        sessions.push(cloned_domain);
    }

    let pool = SessionPool::new(sessions.into_boxed_slice());

    Ok(SslService { pool, system })
}

/// How many IPC sessions the service's pool holds.
///
/// The service accepts a small pool so commands can run concurrently, and the C API takes the
/// size as a plain integer. This is that integer once it has been checked: the bound is the
/// service's, not this crate's, and a caller outside it is told rather than clamped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionCount(u32);

impl SessionCount {
    /// Fewest sessions a pool can be built from.
    pub const MIN: u32 = 1;
    /// Most sessions the service accepts.
    pub const MAX: u32 = 4;

    /// What a caller with no pool sizing of its own should use.
    pub const DEFAULT: Self = Self(3);

    /// The count, as a pool size.
    ///
    /// Exact rather than lossy: the value is bounded by [`MAX`](Self::MAX), which fits every
    /// `usize` this workspace targets.
    const fn to_len(self) -> usize {
        self.0 as usize
    }
}

impl TryFrom<u32> for SessionCount {
    type Error = SessionCountError;

    /// # Errors
    ///
    /// [`SessionCountError`] when `count` falls outside [`MIN`](Self::MIN)`..=`[`MAX`](Self::MAX).
    fn try_from(count: u32) -> Result<Self, Self::Error> {
        match count {
            Self::MIN..=Self::MAX => Ok(Self(count)),
            _ => Err(SessionCountError { count }),
        }
    }
}

/// Error returned when a session count falls outside what the service accepts.
#[derive(Debug, thiserror::Error)]
#[error("session count {count} is outside 1..=4")]
pub struct SessionCountError {
    /// The count that was offered.
    pub count: u32,
}

/// Errors returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    /// SM lookup for the SSL service failed.
    #[error("failed to look up ssl service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Converting the session to a domain failed.
    #[error("failed to ConvertToDomain on ssl session")]
    ConvertToDomain(#[source] ConvertToDomainError),
    /// Cloning the session for the pool failed.
    #[error("failed to clone ssl session for the pool")]
    CloneSession(#[source] nx_sf::service::CloneObjectError),
}
