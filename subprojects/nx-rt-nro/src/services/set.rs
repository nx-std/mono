//! `set:sys` service state and singleton API.
//!
//! This module manages the `set:sys` service session and provides a singleton
//! interface for accessing system settings throughout the application lifecycle.

use nx_service_set::SetSysService;
use nx_std_sync::{
    once_lock::OnceLock,
    rwlock::RwLock,
};

use crate::services::sm;

/// Global `set:sys` state, lazily initialized.
static SET_STATE: OnceLock<RwLock<Option<SetState>>> = OnceLock::new();

/// Returns a reference to the `set:sys` state lock, initializing it if needed.
fn state() -> &'static RwLock<Option<SetState>> {
    SET_STATE.get_or_init(|| RwLock::new(None))
}

/// Initializes the `set:sys` service.
///
/// Selects CMIF or TIPC protocol based on HOS version.
///
/// # Panics
///
/// Panics if SM is not initialized.
pub fn init() -> Result<(), ConnectError> {
    let sm_guard = sm::sm_session();
    let sm = sm_guard.as_ref().expect("SM not initialized");

    // Connect to set:sys service (TIPC on HOS 12.0.0+/Atmosphere, CMIF otherwise)
    let service = if sm::should_use_tipc() {
        nx_service_set::connect_tipc(sm).map_err(ConnectError::Tipc)?
    } else {
        nx_service_set::connect_cmif(sm).map_err(ConnectError::Cmif)?
    };

    let mut guard = state().write();
    *guard = Some(SetState { service });

    Ok(())
}

/// Gets the `set:sys` service.
pub fn get_service() -> Option<impl core::ops::Deref<Target = SetSysService> + 'static> {
    let guard = state().read();
    if guard.is_some() {
        Some(SetServiceRef(guard))
    } else {
        None
    }
}

/// Exits the `set:sys` service.
pub fn exit() {
    let mut guard = state().write();
    // SetSysService is RAII; dropping it closes the session.
    let _ = guard.take();
}

/// Internal storage for `set:sys` service.
struct SetState {
    /// `set:sys` service session
    service: SetSysService,
}

/// Wrapper for accessing SetSysService through RwLockReadGuard.
struct SetServiceRef(nx_std_sync::rwlock::RwLockReadGuard<'static, Option<SetState>>);

impl core::ops::Deref for SetServiceRef {
    type Target = SetSysService;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We only create SetServiceRef when the option is Some
        &self.0.as_ref().unwrap().service
    }
}

/// Error returned by [`init`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to connect using CMIF protocol.
    #[error("failed to connect to set:sys (CMIF)")]
    Cmif(#[source] nx_service_set::ConnectCmifError),
    /// Failed to connect using TIPC protocol.
    #[error("failed to connect to set:sys (TIPC)")]
    Tipc(#[source] nx_service_set::ConnectTipcError),
}

#[cfg(feature = "ffi")]
impl nx_rt_core::error::ToResultCode for ConnectError {
    fn to_rc(self) -> nx_rt_core::error::ResultCode {
        use nx_sf::error::ToResultCode as _;

        match self {
            Self::Cmif(err) => err.to_rc(),
            Self::Tipc(err) => err.to_rc(),
        }
    }
}
