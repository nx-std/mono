//! Service Manager state and singleton API.
//!
//! This module provides centralized storage for SM session state and
//! service overrides. It wraps `nx_service_sm` protocol operations with
//! state management and override support.

pub use nx_service_sm::ConnectError;
use nx_service_sm::SmService;
use nx_sf::{ServiceName, service::Service};
use nx_std_sync::{once_lock::OnceLock, rwlock::RwLock};
use nx_svc::ipc::Handle as SessionHandle;

use crate::env::hos_version::{self, HosVersion};

/// Maximum number of service overrides.
pub const MAX_OVERRIDES: usize = 32;

/// Global SM session.
static SM_SESSION: RwLock<Option<SmService>> = RwLock::new(None);

/// Static override table.
#[allow(clippy::declare_interior_mutable_const)]
static OVERRIDES: [OnceLock<Override>; MAX_OVERRIDES] = {
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

/// Closes the Service Manager connection.
///
/// Releases the SM session. After calling this, other SM functions
/// will fail until [`initialize`] is called again.
pub fn exit() {
    let mut session = SM_SESSION.write();
    if let Some(sm) = session.take() {
        sm.close();
    }
}

/// Gets a service by name, checking overrides first.
///
/// If an override exists for this service name, returns a Service
/// with the override handle (not owned). Otherwise, connects to SM
/// to get the service handle.
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

    // No override, get from SM
    let handle = get_service_handle(name)?;
    Ok(Service::new(handle))
}

/// Gets a service directly from SM (ignoring overrides).
///
/// Returns the raw session handle.
pub fn get_service_handle(name: ServiceName) -> Result<SessionHandle, GetServiceError> {
    let session = SM_SESSION.read();
    let sm = session.as_ref().expect("SM not initialized");
    sm.get_service_handle_cmif(name).map_err(GetServiceError)
}

/// Error returned by [`get_service`] and [`get_service_handle`].
#[derive(Debug, thiserror::Error)]
#[error("protocol error")]
pub struct GetServiceError(#[source] pub nx_service_sm::GetServiceCmifError);

/// Registers a new service with SM.
///
/// Uses CMIF or TIPC based on system version.
pub fn register_service(
    name: ServiceName,
    is_light: bool,
    max_sessions: i32,
) -> Result<SessionHandle, RegisterServiceError> {
    let session = SM_SESSION.read();
    let sm = session.as_ref().expect("SM not initialized");

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
) -> Result<SessionHandle, RegisterServiceError> {
    let session = SM_SESSION.read();
    let sm = session.as_ref().expect("SM not initialized");
    sm.register_service_cmif(name, is_light, max_sessions)
        .map_err(RegisterServiceError::Cmif)
}

/// Registers a service using TIPC protocol.
pub fn register_service_tipc(
    name: ServiceName,
    is_light: bool,
    max_sessions: i32,
) -> Result<SessionHandle, RegisterServiceError> {
    let session = SM_SESSION.read();
    let sm = session.as_ref().expect("SM not initialized");
    sm.register_service_tipc(name, is_light, max_sessions)
        .map_err(RegisterServiceError::Tipc)
}

/// Error returned by [`register_service`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterServiceError {
    /// CMIF protocol error.
    #[error("CMIF protocol error")]
    Cmif(#[source] nx_service_sm::RegisterServiceCmifError),
    /// TIPC protocol error.
    #[error("TIPC protocol error")]
    Tipc(#[source] nx_service_sm::RegisterServiceTipcError),
}

/// Unregisters a service from SM.
///
/// Uses CMIF or TIPC based on system version.
pub fn unregister_service(name: ServiceName) -> Result<(), UnregisterServiceError> {
    let session = SM_SESSION.read();
    let sm = session.as_ref().expect("SM not initialized");

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
    let session = SM_SESSION.read();
    let sm = session.as_ref().expect("SM not initialized");
    sm.unregister_service_cmif(name)
        .map_err(UnregisterServiceError::Cmif)
}

/// Unregisters a service using TIPC protocol.
pub fn unregister_service_tipc(name: ServiceName) -> Result<(), UnregisterServiceError> {
    let session = SM_SESSION.read();
    let sm = session.as_ref().expect("SM not initialized");
    sm.unregister_service_tipc(name)
        .map_err(UnregisterServiceError::Tipc)
}

/// Error returned by [`unregister_service`].
#[derive(Debug, thiserror::Error)]
pub enum UnregisterServiceError {
    /// CMIF protocol error.
    #[error("CMIF protocol error")]
    Cmif(#[source] nx_service_sm::UnregisterServiceCmifError),
    /// TIPC protocol error.
    #[error("TIPC protocol error")]
    Tipc(#[source] nx_service_sm::UnregisterServiceTipcError),
}

/// Detaches the current SM client session.
///
/// Only available on HOS 11.0.0-11.0.1 (CMIF) or Atmosphere (TIPC).
pub fn detach_client() -> Result<(), DetachClientError> {
    let session = SM_SESSION.read();
    let sm = session.as_ref().expect("SM not initialized");

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
    let session = SM_SESSION.read();
    let sm = session.as_ref().expect("SM not initialized");
    sm.detach_client_cmif().map_err(DetachClientError::Cmif)
}

/// Detaches using TIPC protocol.
pub fn detach_client_tipc() -> Result<(), DetachClientError> {
    let session = SM_SESSION.read();
    let sm = session.as_ref().expect("SM not initialized");
    sm.detach_client_tipc().map_err(DetachClientError::Tipc)
}

/// Error returned by [`detach_client`].
#[derive(Debug, thiserror::Error)]
pub enum DetachClientError {
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

/// Returns a read guard to the SM session.
#[inline]
pub fn sm_session() -> nx_std_sync::rwlock::RwLockReadGuard<'static, Option<SmService>> {
    SM_SESSION.read()
}

/// Returns a write guard to the SM session.
#[inline]
pub fn sm_session_mut() -> nx_std_sync::rwlock::RwLockWriteGuard<'static, Option<SmService>> {
    SM_SESSION.write()
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
