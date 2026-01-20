//! Time service state and singleton API.
//!
//! This module manages the Time service session and provides a singleton interface
//! for accessing time functionality throughout the application lifecycle.

use nx_service_time::{TimeService, TimeServiceType};
use nx_std_sync::{once_lock::OnceLock, rwlock::RwLock};

use crate::service_manager;

/// Global Time state, lazily initialized.
static TIME_STATE: OnceLock<RwLock<Option<TimeState>>> = OnceLock::new();

/// Returns a reference to the Time state lock, initializing it if needed.
fn state() -> &'static RwLock<Option<TimeState>> {
    TIME_STATE.get_or_init(|| RwLock::new(None))
}

/// Initializes the Time service.
///
/// This matches libnx's `timeInitialize()` behavior, which connects to the
/// time:u service by default.
///
/// # Panics
///
/// Panics if SM is not initialized.
pub fn init() -> Result<(), ConnectError> {
    let sm_guard = service_manager::sm_session();
    let sm = sm_guard.as_ref().expect("SM not initialized");

    // Connect to Time service (time:u by default)
    let service =
        nx_service_time::connect(sm, TimeServiceType::User).map_err(ConnectError::Connect)?;

    let mut guard = state().write();
    *guard = Some(TimeState { service });

    Ok(())
}

/// Gets the Time service.
pub fn get_service() -> Option<impl core::ops::Deref<Target = TimeService> + 'static> {
    let guard = state().read();
    if guard.is_some() {
        Some(TimeServiceRef(guard))
    } else {
        None
    }
}

/// Exits the Time service.
pub fn exit() {
    let mut guard = state().write();
    if let Some(time_state) = guard.take() {
        time_state.service.close();
    }
}

/// Internal storage for Time service.
struct TimeState {
    /// Time service (IStaticService with clock and timezone services)
    service: TimeService,
}

/// Wrapper for accessing TimeService through RwLockReadGuard.
struct TimeServiceRef(nx_std_sync::rwlock::RwLockReadGuard<'static, Option<TimeState>>);

impl core::ops::Deref for TimeServiceRef {
    type Target = TimeService;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We only create TimeServiceRef when the option is Some
        &self.0.as_ref().unwrap().service
    }
}

/// Error returned by [`init`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to connect to Time service.
    #[error("failed to connect to Time service")]
    Connect(#[source] nx_service_time::ConnectError),
}
