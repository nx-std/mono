//! Service Manager state and singleton API.
//!
//! This module provides centralized storage for SM session state and
//! service overrides. It wraps `nx_service_sm` protocol operations with
//! state management and override support.

use core::ops::Deref;

pub use nx_service_sm::ConnectError;
use nx_service_sm::SmService;
#[cfg(feature = "ffi")]
use nx_sf::error::ToResultCode as _;
#[cfg(feature = "ffi")]
use nx_sf::ffi::Service;
use nx_sf::{
    ServiceName,
    service::OwnedSessionHandle,
};
use nx_std_sync::{
    once_lock::OnceLock,
    rwlock::{
        RwLock,
        RwLockReadGuard,
    },
};
#[cfg(feature = "ffi")]
use nx_svc::error::ResultCode;
use nx_svc::ipc::Handle as SessionHandle;

use crate::env::hos_version::{
    self,
    HosVersion,
};
#[cfg(feature = "ffi")]
use crate::error::{
    LibnxError,
    ToResultCode,
    libnx_error,
};

/// Maximum number of service overrides.
pub const MAX_OVERRIDES: usize = 32;

/// Global SM session.
static SM_SESSION: RwLock<Option<SmService>> = RwLock::new(None);

/// Static override table.
static OVERRIDES: [OnceLock<Override>; MAX_OVERRIDES] = {
    // The table is fixed-size and every slot starts empty, so the array is
    // built by repeating one empty cell. Repeating a `const` is the only way
    // to write that for a type without `Copy`, and the interior mutability the
    // lint warns about is the point: each slot is filled at most once, later,
    // through `OnceLock`.
    #[expect(
        clippy::declare_interior_mutable_const,
        reason = "the const is a template for array repetition, never read as a value"
    )]
    const INIT: OnceLock<Override> = OnceLock::new();
    [INIT; MAX_OVERRIDES]
};

/// A service override entry.
struct Override {
    name: ServiceName,
    handle: SessionHandle,
}

/// Returns whether TIPC should be used for RegisterService/UnregisterService.
///
/// TIPC is used on Atmosphere or HOS 12.0.0+.
#[inline]
pub fn should_use_tipc() -> bool {
    hos_version::is_atmosphere() || hos_version::get() >= HosVersion::new(12, 0, 0)
}

/// Initializes the Service Manager connection.
///
/// Connects to SM and stores the session for future use.
/// Thread-safe: only the first call performs initialization.
pub fn initialize() -> Result<(), InitializeError> {
    // Check if already initialized
    {
        let session = SM_SESSION.read();
        if session.is_some() {
            return Ok(());
        }
    }

    // Try to initialize
    let mut session = SM_SESSION.write();

    // Double-check after acquiring write lock
    if session.is_some() {
        return Ok(());
    }

    // Connect to SM
    let sm = nx_service_sm::connect().map_err(InitializeError)?;

    // Store the session
    *session = Some(sm);

    Ok(())
}

/// Error returned by [`initialize`].
///
/// Failed to connect to SM.
#[derive(Debug, thiserror::Error)]
#[error("failed to connect to SM")]
pub struct InitializeError(#[source] pub ConnectError);

#[cfg(feature = "ffi")]
impl ToResultCode for InitializeError {
    fn to_rc(self) -> ResultCode {
        self.0.to_rc()
    }
}

/// Closes the Service Manager connection.
///
/// Releases the SM session. After calling this, other SM functions
/// will fail until [`initialize`] is called again.
pub fn exit() {
    let mut session = SM_SESSION.write();
    // `SmService` is RAII: drop closes the underlying session.
    let _ = session.take();
}

/// Gets a service by name, checking overrides first.
///
/// If an override exists for this service name, returns a Service
/// with the override handle (not owned). Otherwise, connects to SM
/// to get the service handle.
#[cfg(feature = "ffi")]
pub fn get_service(name: ServiceName) -> Result<Service, GetServiceError> {
    // Check for override first
    if let Some(handle) = get_override(name) {
        // Override service: own_handle = 0 (don't close on drop)
        return Ok(Service {
            session: handle,
            own_handle: 0, // Don't own the override handle
            object_id: 0,
            pointer_buffer_size: 0,
        });
    }

    // No override, get from SM. Construct an owned-mode FFI service:
    // own_handle = 1, object_id = 0, pointer_buffer_size = 0 (queried later
    // by the C caller if needed).
    let handle = get_service_handle(name)?;
    Ok(Service {
        // Ownership passes into the C-owned `Service`, which closes the handle itself.
        session: handle.into_handle(),
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    })
}

/// Gets a service directly from SM (ignoring overrides).
///
/// Returns the raw session handle.
pub fn get_service_handle(name: ServiceName) -> Result<OwnedSessionHandle, GetServiceError> {
    let sm = session()?;
    sm.get_service_handle_cmif(name)
        .map_err(GetServiceError::Protocol)
}

/// Error returned by [`get_service`] and [`get_service_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetServiceError {
    /// No Service Manager session is open.
    ///
    /// Occurs when the lookup is attempted before the session is opened or
    /// after it is closed. Nothing was sent.
    #[error("the Service Manager is not initialized")]
    NotInitialized,
    /// The server refused the lookup, or the reply could not be decoded.
    ///
    /// Occurs when the name is not registered, or the caller is not permitted
    /// to reach it. No handle was issued.
    #[error("protocol error")]
    Protocol(#[source] nx_service_sm::GetServiceCmifError),
}

impl From<NotInitializedError> for GetServiceError {
    fn from(_: NotInitializedError) -> Self {
        Self::NotInitialized
    }
}

#[cfg(feature = "ffi")]
impl ToResultCode for GetServiceError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::NotInitialized => libnx_error(LibnxError::NotInitialized),
            Self::Protocol(err) => err.to_rc(),
        }
    }
}

/// Registers a new service with SM.
///
/// Uses CMIF or TIPC based on system version.
pub fn register_service(
    name: ServiceName,
    is_light: bool,
    max_sessions: i32,
) -> Result<OwnedSessionHandle, RegisterServiceError> {
    let sm = session()?;

    if should_use_tipc() {
        sm.register_service_tipc(name, is_light, max_sessions)
            .map_err(RegisterServiceError::Tipc)
    } else {
        sm.register_service_cmif(name, is_light, max_sessions)
            .map_err(RegisterServiceError::Cmif)
    }
}

/// Registers a service using CMIF protocol.
pub fn register_service_cmif(
    name: ServiceName,
    is_light: bool,
    max_sessions: i32,
) -> Result<OwnedSessionHandle, RegisterServiceError> {
    let sm = session()?;
    sm.register_service_cmif(name, is_light, max_sessions)
        .map_err(RegisterServiceError::Cmif)
}

/// Registers a service using TIPC protocol.
pub fn register_service_tipc(
    name: ServiceName,
    is_light: bool,
    max_sessions: i32,
) -> Result<OwnedSessionHandle, RegisterServiceError> {
    let sm = session()?;
    sm.register_service_tipc(name, is_light, max_sessions)
        .map_err(RegisterServiceError::Tipc)
}

/// Error returned by [`register_service`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterServiceError {
    /// No Service Manager session is open.
    ///
    /// Occurs when the registration is attempted before the session is opened or
    /// after it is closed. Nothing was sent.
    #[error("the Service Manager is not initialized")]
    NotInitialized,
    /// CMIF protocol error.
    #[error("CMIF protocol error")]
    Cmif(#[source] nx_service_sm::RegisterServiceCmifError),
    /// TIPC protocol error.
    #[error("TIPC protocol error")]
    Tipc(#[source] nx_service_sm::RegisterServiceTipcError),
}

impl From<NotInitializedError> for RegisterServiceError {
    fn from(_: NotInitializedError) -> Self {
        Self::NotInitialized
    }
}

#[cfg(feature = "ffi")]
impl ToResultCode for RegisterServiceError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::NotInitialized => libnx_error(LibnxError::NotInitialized),
            Self::Cmif(err) => err.to_rc(),
            Self::Tipc(err) => err.to_rc(),
        }
    }
}

/// Unregisters a service from SM.
///
/// Uses CMIF or TIPC based on system version.
pub fn unregister_service(name: ServiceName) -> Result<(), UnregisterServiceError> {
    let sm = session()?;

    if should_use_tipc() {
        sm.unregister_service_tipc(name)
            .map_err(UnregisterServiceError::Tipc)
    } else {
        sm.unregister_service_cmif(name)
            .map_err(UnregisterServiceError::Cmif)
    }
}

/// Unregisters a service using CMIF protocol.
pub fn unregister_service_cmif(name: ServiceName) -> Result<(), UnregisterServiceError> {
    let sm = session()?;
    sm.unregister_service_cmif(name)
        .map_err(UnregisterServiceError::Cmif)
}

/// Unregisters a service using TIPC protocol.
pub fn unregister_service_tipc(name: ServiceName) -> Result<(), UnregisterServiceError> {
    let sm = session()?;
    sm.unregister_service_tipc(name)
        .map_err(UnregisterServiceError::Tipc)
}

/// Error returned by [`unregister_service`].
#[derive(Debug, thiserror::Error)]
pub enum UnregisterServiceError {
    /// No Service Manager session is open.
    ///
    /// Occurs when the deregistration is attempted before the session is opened or
    /// after it is closed. Nothing was sent.
    #[error("the Service Manager is not initialized")]
    NotInitialized,
    /// CMIF protocol error.
    #[error("CMIF protocol error")]
    Cmif(#[source] nx_service_sm::UnregisterServiceCmifError),
    /// TIPC protocol error.
    #[error("TIPC protocol error")]
    Tipc(#[source] nx_service_sm::UnregisterServiceTipcError),
}

impl From<NotInitializedError> for UnregisterServiceError {
    fn from(_: NotInitializedError) -> Self {
        Self::NotInitialized
    }
}

#[cfg(feature = "ffi")]
impl ToResultCode for UnregisterServiceError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::NotInitialized => libnx_error(LibnxError::NotInitialized),
            Self::Cmif(err) => err.to_rc(),
            Self::Tipc(err) => err.to_rc(),
        }
    }
}

/// Detaches the current SM client session.
///
/// Only available on HOS 11.0.0-11.0.1 (CMIF) or Atmosphere (TIPC).
pub fn detach_client() -> Result<(), DetachClientError> {
    let sm = session()?;

    if hos_version::is_atmosphere() {
        sm.detach_client_tipc().map_err(DetachClientError::Tipc)
    } else if hos_version::get() >= HosVersion::new(11, 0, 0)
        && hos_version::get() < HosVersion::new(12, 0, 0)
    {
        sm.detach_client_cmif().map_err(DetachClientError::Cmif)
    } else {
        Err(DetachClientError::IncompatibleVersion)
    }
}

/// Detaches using CMIF protocol.
pub fn detach_client_cmif() -> Result<(), DetachClientError> {
    let sm = session()?;
    sm.detach_client_cmif().map_err(DetachClientError::Cmif)
}

/// Detaches using TIPC protocol.
pub fn detach_client_tipc() -> Result<(), DetachClientError> {
    let sm = session()?;
    sm.detach_client_tipc().map_err(DetachClientError::Tipc)
}

/// Error returned by [`detach_client`].
#[derive(Debug, thiserror::Error)]
pub enum DetachClientError {
    /// No Service Manager session is open.
    ///
    /// Occurs when the detach is attempted before the session is opened or
    /// after it is closed. Nothing was sent.
    #[error("the Service Manager is not initialized")]
    NotInitialized,
    /// Detach is not supported on this system version.
    #[error("incompatible system version")]
    IncompatibleVersion,
    /// CMIF protocol error.
    #[error("CMIF protocol error")]
    Cmif(#[source] nx_service_sm::DetachClientCmifError),
    /// TIPC protocol error.
    #[error("TIPC protocol error")]
    Tipc(#[source] nx_service_sm::DetachClientTipcError),
}

impl From<NotInitializedError> for DetachClientError {
    fn from(_: NotInitializedError) -> Self {
        Self::NotInitialized
    }
}

#[cfg(feature = "ffi")]
impl ToResultCode for DetachClientError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::NotInitialized => libnx_error(LibnxError::NotInitialized),
            // A refusal this layer decides, so it reports a result code of its
            // own: no request went out and no server named one.
            Self::IncompatibleVersion => libnx_error(LibnxError::IncompatSysVer),
            Self::Cmif(err) => err.to_rc(),
            Self::Tipc(err) => err.to_rc(),
        }
    }
}

/// Borrows the process's Service Manager session.
///
/// A process gets one session, and this module holds it. Callers borrow it for
/// the length of a request rather than being handed the container it lives in,
/// so "is it open?" is answered once, here, instead of at every call site.
///
/// # Errors
///
/// Returns an error when no session is open: either [`initialize`] has not run
/// yet, or [`exit`] has already closed it. Nothing was sent.
#[inline]
pub fn session() -> Result<SmSession, NotInitializedError> {
    let guard = SM_SESSION.read();
    if guard.is_none() {
        return Err(NotInitializedError);
    }
    Ok(SmSession(guard))
}

/// A borrow of the Service Manager session.
///
/// Holds the read lock, so the session cannot be closed while it is in use.
pub struct SmSession(RwLockReadGuard<'static, Option<SmService>>);

impl Deref for SmSession {
    type Target = SmService;

    fn deref(&self) -> &Self::Target {
        match self.0.as_ref() {
            Some(sm) => sm,
            // SAFETY: `session` is the only constructor, and it returns an
            // error rather than a guard when the session is closed. The read
            // lock this holds keeps it open for the borrow's lifetime.
            None => unsafe { core::hint::unreachable_unchecked() },
        }
    }
}

/// Error returned by [`session`].
#[derive(Debug, thiserror::Error)]
#[error("the Service Manager is not initialized")]
pub struct NotInitializedError;

#[cfg(feature = "ffi")]
impl ToResultCode for NotInitializedError {
    fn to_rc(self) -> ResultCode {
        libnx_error(LibnxError::NotInitialized)
    }
}

/// Registers a pre-connected service handle that bypasses SM lookup.
///
/// After registration, [`get_override`] returns this handle for the given name.
/// Typically called during early initialization from homebrew loader config.
pub fn add_override(name: ServiceName, handle: SessionHandle) -> Result<(), TooManyOverridesError> {
    for slot in &OVERRIDES {
        if slot.set(Override { name, handle }).is_ok() {
            return Ok(());
        }
    }

    Err(TooManyOverridesError)
}

/// Error returned when the override table is full.
#[derive(Debug, thiserror::Error)]
#[error("too many overrides (max 32)")]
pub struct TooManyOverridesError;

#[cfg(feature = "ffi")]
impl ToResultCode for TooManyOverridesError {
    fn to_rc(self) -> ResultCode {
        // libnx aborts with this code from `smAddOverrideHandle` rather than
        // returning it; the description is the right one either way.
        libnx_error(LibnxError::TooManyOverrides)
    }
}

/// Gets an override handle for a service name, or `None` if no override exists.
#[inline]
pub fn get_override(name: ServiceName) -> Option<SessionHandle> {
    let target = name.to_u64();

    for slot in &OVERRIDES {
        if let Some(entry) = slot.get()
            && entry.name.to_u64() == target
        {
            return Some(entry.handle);
        }
    }

    None
}
