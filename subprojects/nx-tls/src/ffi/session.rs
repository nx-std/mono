//! The process-wide connection to the `ssl` service.
//!
//! No C entry point passes a service in: `sslCreateContext` names a version and nothing else, and
//! a context reaches its own commands through the struct the caller holds. So the connection is
//! held here, in one slot for the whole process, and every operation borrows it for the length of
//! one command.
//!
//! ## Reference counted, because the C API is
//!
//! `sslInitialize` and `sslExit` are a matched pair a program may nest, which upstream supports
//! with a mutex and a count. [`initialize`] and [`exit`] are that pair: the first call connects,
//! the last disconnects, and the ones in between move the count. A second [`initialize`] with a
//! different session count keeps the connection the first one built, which is upstream's behaviour
//! and the only one that can be right when the sessions are already open.
//!
//! ## The lock is read-mostly for a reason
//!
//! Only [`initialize`] and [`exit`] write, and they run once each. Everything else takes a read
//! guard, so commands run concurrently: that is what the session pool inside [`SslService`] is
//! for, and a program handshaking on one thread while another reads depends on it.
//!
//! A blocking read therefore holds a read guard for as long as it blocks, which is unbounded. That
//! is deliberate: it blocks a concurrent [`exit`], and tearing the service down underneath an
//! in-flight command is precisely what must not happen.
//!
//! ## Why there is no `extern-state`
//!
//! A process-wide `static` normally needs the `extern-state` treatment, so a program linking a
//! second static library does not get a second copy of it. This one does not, because nothing can
//! reach it from Rust: the module is private and gated on `ffi`, so the only route to this
//! connection is the C surface, which is single by construction. If this crate ever grows a Rust
//! API worth having, that API and the feature arrive together.

use nx_rt_core::services::sm;
use nx_service_ssl::{
    ConnectCmifError,
    SessionCount,
    SslService,
    connect_cmif,
};
use nx_sf::{
    error::{
        LibnxError,
        ToResultCode as _,
        libnx_error,
    },
    ffi::Service,
    service::DispatchError,
};
use nx_std_sync::rwlock::RwLock;

use super::firmware;

/// The process-wide `ssl` connection, and the count of callers that asked for it.
static STATE: RwLock<SslState> = RwLock::new(SslState::CLOSED);

/// Everything the C surface needs to know about whether the service is up.
///
/// The count and the connection sit under one lock because they are one fact: the connection is
/// established exactly while the count is above zero, and splitting them would let the two
/// disagree.
struct SslState {
    /// How many `sslInitialize` calls have not yet been matched by an `sslExit`.
    ref_count: u32,
    /// The connection, established while `ref_count` is non-zero.
    service: Option<SslService>,
    /// The domain root, in the shape `sslGetServiceSession` hands out.
    session: Service,
}

impl SslState {
    /// What the process starts with, and returns to when the last caller exits.
    const CLOSED: Self = Self {
        ref_count: 0,
        service: None,
        session: Service {
            session: nx_svc::ipc::Handle::from_raw_unchecked(nx_svc::raw::INVALID_HANDLE),
            own_handle: 0,
            object_id: 0,
            pointer_buffer_size: 0,
        },
    };
}

/// Establishes the connection, or records another caller for one already established.
///
/// `sessions` sizes the pool, and `system` selects the system service variant. Neither is
/// consulted when a connection already exists.
///
/// # Errors
///
/// Returns [`InitializeError`] when this call was the one that had to connect and the handshake
/// failed. Nothing is stored in that case and the count is left as it was, so the call can be
/// retried.
pub(crate) fn initialize(sessions: SessionCount, system: bool) -> Result<(), InitializeError> {
    let mut state = STATE.write();

    if state.ref_count > 0 {
        state.ref_count += 1;
        return Ok(());
    }

    let service = connect(sessions, system)?;
    state.session = describe_root(&service);
    state.service = Some(service);
    state.ref_count = 1;
    Ok(())
}

/// Errors returned by [`initialize`].
#[derive(Debug, thiserror::Error)]
pub(crate) enum InitializeError {
    /// The runtime holds no service manager session to look the service up through.
    #[error("the service manager session is not open")]
    NoServiceManager,

    /// The `ssl` handshake failed.
    #[error("failed to connect to the ssl service")]
    Connect(#[source] ConnectCmifError),

    /// Declaring the interface revision failed.
    #[error("failed to declare the ssl interface revision")]
    InterfaceVersion(#[source] DispatchError),
}

impl nx_sf::error::ToResultCode for InitializeError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            // A runtime that has not opened the service manager session is one this ran before,
            // which is what upstream reports for a subsystem used before its hook.
            Self::NoServiceManager => libnx_error(LibnxError::NotInitialized),
            Self::Connect(err) => connect_error(err),
            Self::InterfaceVersion(err) => err.to_rc(),
        }
    }
}

/// Reports a failed handshake as the stage that failed.
///
/// Each stage carries the error its own client reported, and forwarding that is what lets a caller
/// see which of the four steps refused rather than one code for all of them.
fn connect_error(err: ConnectCmifError) -> nx_sf::error::ResultCode {
    match err {
        ConnectCmifError::GetService(err) => err.to_rc(),
        ConnectCmifError::ConvertToDomain(err) => err.to_rc(),
        ConnectCmifError::CloneSession(err) => err.to_rc(),
    }
}

/// Opens the connection and brings it to the state a command may be sent on.
///
/// The interface revision is declared here rather than left to the caller, because the service
/// answers later commands differently depending on it: a connection where this was skipped is not
/// the same connection, so it is part of establishing one rather than a thing to do afterwards.
fn connect(sessions: SessionCount, system: bool) -> Result<SslService, InitializeError> {
    let sm = sm::session().map_err(|_| InitializeError::NoServiceManager)?;

    let service = connect_cmif(&sm, system, sessions).map_err(InitializeError::Connect)?;

    if let Some(version) = firmware::interface_version() {
        service
            .set_interface_version(version)
            .map_err(InitializeError::InterfaceVersion)?;
    }

    Ok(service)
}

/// Describes the domain root in the shape a C caller reads.
///
/// `own_handle` is set, and with a non-zero object id that is the encoding for a domain root:
/// which is what this is. It says the struct names a session somebody owns, and the somebody is
/// the pool, not the caller.
fn describe_root(service: &SslService) -> Service {
    let root = service.root();

    Service {
        session: root.handle().to_handle(),
        own_handle: 1,
        object_id: root.object_id().to_raw(),
        pointer_buffer_size: root.pointer_buffer_size(),
    }
}

/// Releases one caller's claim, disconnecting when it was the last.
///
/// Blocks until every in-flight command has finished, because tearing the service down under one
/// would close the sessions its response is due on. Does nothing when no connection is
/// established, so it is safe to call twice and safe to call after a failed [`initialize`].
pub(crate) fn exit() {
    let mut state = STATE.write();

    if state.ref_count == 0 {
        return;
    }

    state.ref_count -= 1;
    if state.ref_count == 0 {
        // Dropping the service closes every session in the pool, and with them every context and
        // connection the server opened against the domain.
        state.service = None;
        state.session = SslState::CLOSED.session;
    }
}

/// Runs `command` against the connection, if one is established.
///
/// Returns `None` when the service is not up, which is what every C entry point answers with
/// `LibnxError_NotInitialized`.
pub(crate) fn with_service<T>(command: impl FnOnce(&SslService) -> T) -> Option<T> {
    let state = STATE.read();
    state.service.as_ref().map(command)
}

/// The address of the struct describing the domain root.
///
/// The C API hands out the service session so a program can send commands this crate does not
/// carry, and a pointer is what it hands out. The address is inside a `static`, so it stays valid
/// for the life of the process and does not depend on the guard taken to compute it.
///
/// What the pointer names is only meaningful while the service is up: before [`initialize`] and
/// after the last [`exit`] the struct is zeroed, which is what upstream leaves behind too. Reading
/// it races with a concurrent [`exit`] in both implementations, and neither can prevent that: the
/// C caller holds a bare pointer, and no lock outlives the call that returned it.
pub(crate) fn root_session() -> *mut Service {
    let state = STATE.read();
    (&raw const state.session).cast_mut()
}
