//! Service Manager (`sm`) protocol implementation.
//!
//! The Service Manager is how a process reaches everything else on Horizon OS.
//! Services — filesystem, display, input, audio, and the rest — are not found
//! by address or by linking: a process asks SM for one *by name* and receives
//! a session it can talk to. SM is the directory that makes Horizon's
//! service-oriented system navigable, and it is in place from early in boot.
//!
//! Two roles use it. A *client* resolves a service name into a usable session.
//! A *service host* publishes a name so that those lookups can succeed. Either
//! way the first step is the same — connect to SM and register as a client;
//! only then does SM answer requests.
//!
//! This crate exposes stateless SM operations as methods on [`SmService`], the
//! session returned by [`connect`]. It keeps no global state and never probes
//! the system version, so the caller chooses the protocol per call.
//!
//! ## Connecting
//!
//! [`connect`] performs the handshake every process begins with: it reaches
//! the `sm:` port — retrying while SM is still coming up during boot — and
//! registers the caller as a client. The returned [`SmService`] is then ready
//! for lookups and registrations.
//!
//! ## Operations
//!
//! Beyond that initial registration, [`SmService`] lets a caller:
//!
//! - **resolve a service** — turn a [`ServiceName`] into a session handle for
//!   it; the everyday client operation.
//! - **register / unregister a service** — publish or withdraw a name, and
//!   obtain the port a host accepts sessions on.
//! - **detach** — release the caller's own client registration.
//!
//! ## Protocol Support
//!
//! SM speaks two IPC protocols: the original CMIF and the newer TIPC, which
//! `sm` gained in `[12.0.0]` and Atmosphère speaks as well. Every operation
//! comes in a `_cmif` and a `_tipc` variant; choosing the one that matches the
//! running system is the caller's responsibility.
//!
//! ## Hosversion variants
//!
//! Detaching a client became possible only in `[11.0.0]`, and its reach is
//! narrow: [`SmService::detach_client_cmif`] works on `[11.0.0]`-`[11.0.1]`,
//! while [`SmService::detach_client_tipc`] is an Atmosphère extension that
//! stock SM does not provide.

#![no_std]

extern crate nx_panic_handler; // Provide #![panic_handler]

pub use nx_sf::ServiceName;
use nx_sf::{
    error::ToResultCode,
    service::{BorrowedSessionHandle, OwnedSessionHandle, Session},
};
use nx_svc::{
    error::{ResultCode, ToResultCode as _},
    ipc::{self},
};

mod cmif;
mod proto;
mod tipc;

pub use self::{
    cmif::{
        DetachClientError as DetachClientCmifError, GetServiceError as GetServiceCmifError,
        RegisterClientError as RegisterClientCmifError,
        RegisterServiceError as RegisterServiceCmifError,
        UnregisterServiceError as UnregisterServiceCmifError,
    },
    proto::SM_PORT_NAME,
    tipc::{
        DetachClientError as DetachClientTipcError, GetServiceError as GetServiceTipcError,
        RegisterClientError as RegisterClientTipcError,
        RegisterServiceError as RegisterServiceTipcError,
        UnregisterServiceError as UnregisterServiceTipcError,
    },
};

/// Sleep duration (in nanoseconds) when SM is not available during connection.
const CONNECT_RETRY_SLEEP_NS: u64 = 50_000_000; // 50ms

/// Service Manager session wrapper.
///
/// Provides type safety to distinguish SM sessions from regular services.
#[repr(transparent)]
pub struct SmService(Session);

impl SmService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl SmService {
    /// Gets a service handle by name using CMIF protocol.
    #[inline]
    pub fn get_service_handle_cmif(
        &self,
        name: ServiceName,
    ) -> Result<OwnedSessionHandle, GetServiceCmifError> {
        cmif::get_service_handle(self.0.handle(), name)
    }

    /// Registers a service using CMIF protocol.
    #[inline]
    pub fn register_service_cmif(
        &self,
        name: ServiceName,
        is_light: bool,
        max_sessions: i32,
    ) -> Result<OwnedSessionHandle, RegisterServiceCmifError> {
        cmif::register_service(self.0.handle(), name, is_light, max_sessions)
    }

    /// Unregisters a service using CMIF protocol.
    #[inline]
    pub fn unregister_service_cmif(
        &self,
        name: ServiceName,
    ) -> Result<(), UnregisterServiceCmifError> {
        cmif::unregister_service(self.0.handle(), name)
    }

    /// Detaches the client using CMIF protocol.
    ///
    /// Only available on HOS 11.0.0-11.0.1.
    #[inline]
    pub fn detach_client_cmif(&self) -> Result<(), DetachClientCmifError> {
        cmif::detach_client(self.0.handle())
    }
}

/// TIPC protocol methods.
///
/// Requires HOS 12.0.0+ or Atmosphere.
impl SmService {
    /// Gets a service handle by name using TIPC protocol.
    ///
    /// Requires HOS 12.0.0+ or Atmosphere.
    #[inline]
    pub fn get_service_handle_tipc(
        &self,
        name: ServiceName,
    ) -> Result<OwnedSessionHandle, GetServiceTipcError> {
        tipc::get_service_handle(self.0.handle(), name)
    }

    /// Registers a service using TIPC protocol.
    #[inline]
    pub fn register_service_tipc(
        &self,
        name: ServiceName,
        is_light: bool,
        max_sessions: i32,
    ) -> Result<OwnedSessionHandle, RegisterServiceTipcError> {
        tipc::register_service(self.0.handle(), name, is_light, max_sessions)
    }

    /// Unregisters a service using TIPC protocol.
    #[inline]
    pub fn unregister_service_tipc(
        &self,
        name: ServiceName,
    ) -> Result<(), UnregisterServiceTipcError> {
        tipc::unregister_service(self.0.handle(), name)
    }

    /// Detaches the client using TIPC protocol.
    ///
    /// Only available on Atmosphere.
    #[inline]
    pub fn detach_client_tipc(&self) -> Result<(), DetachClientTipcError> {
        tipc::detach_client(self.0.handle())
    }

    /// Registers the client using TIPC protocol.
    ///
    /// [`connect`] already registers over CMIF, which is the path that works on
    /// every firmware; this is for a caller that has to re-register over TIPC.
    #[inline]
    pub fn register_client_tipc(&self) -> Result<(), RegisterClientTipcError> {
        tipc::register_client(self.0.handle())
    }
}

/// Connects to the _Service Manager_.
///
/// Connects to the "sm:" named port and registers as a client, retrying
/// until the port becomes available (with 50ms sleep between attempts).
///
/// Returns an [`SmService`] that can be used for SM operations.
pub fn connect() -> Result<SmService, ConnectError> {
    // Connect to "sm:" named port, retrying on NotFound
    let handle = loop {
        match ipc::connect_to_named_port(SM_PORT_NAME) {
            Ok(handle) => break handle,
            Err(ipc::ConnectError::NotFound) => {
                // SM not yet available, wait and retry
                nx_svc::thread::sleep(CONNECT_RETRY_SLEEP_NS);
            }
            Err(err) => return Err(ConnectError::Connect(err)),
        }
    };

    // SAFETY: The port connect above returned a freshly opened session that this process owns
    // and nothing else closes, so this is where its single owner is established.
    let handle = OwnedSessionHandle::from_handle_unchecked(handle);

    // Send RegisterClient (command 0) via CMIF with send_pid=true.
    // pointer_buffer_size is 0 because SM doesn't use pointer buffers.
    cmif::register_client(handle.as_borrowed()).map_err(ConnectError::RegisterClient)?;

    // The handle adopted above moves into the `Session`, which owns it from here.
    Ok(SmService(Session::new(handle, 0)))
}

/// Error returned by [`connect`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to connect to the "sm:" named port.
    #[error("failed to connect to sm:")]
    Connect(#[source] ipc::ConnectError),
    /// Failed to register client with SM.
    #[error("failed to register client")]
    RegisterClient(#[source] cmif::RegisterClientError),
}

impl ToResultCode for ConnectError {
    fn to_rc(self) -> ResultCode {
        match self {
            // The kernel rejected the port connection, so it owns this code
            // and it resolves through `nx-svc`'s trait rather than `nx-sf`'s.
            Self::Connect(err) => err.to_rc(),
            Self::RegisterClient(err) => err.to_rc(),
        }
    }
}
