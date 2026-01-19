//! APM service state and singleton API.
//!
//! This module manages the APM service session and provides a singleton interface
//! for accessing APM functionality throughout the application lifecycle.

use nx_service_apm::{ApmService, ApmSession};
use nx_std_sync::{once_lock::OnceLock, rwlock::RwLock};

use crate::service_manager;

/// Global APM state, lazily initialized.
static APM_STATE: OnceLock<RwLock<Option<ApmState>>> = OnceLock::new();

/// Returns a reference to the APM state lock, initializing it if needed.
fn state() -> &'static RwLock<Option<ApmState>> {
    APM_STATE.get_or_init(|| RwLock::new(None))
}

/// Initializes the APM service and opens ISession.
///
/// This matches libnx's `apmInitialize()` behavior, which connects to the
/// service and immediately opens the ISession interface.
///
/// # Panics
///
/// Panics if SM is not initialized.
pub fn init() -> Result<(), ConnectError> {
    let sm_guard = service_manager::sm_session();
    let sm = sm_guard.as_ref().expect("SM not initialized");

    // Connect to APM service
    let service = nx_service_apm::connect(sm).map_err(ConnectError::Connect)?;

    // Open session immediately (libnx compatibility)
    let session = service.open_session().map_err(ConnectError::OpenSession)?;

    let mut guard = state().write();
    *guard = Some(ApmState { service, session });

    Ok(())
}

/// Gets the APM service.
pub fn get_service() -> Option<impl core::ops::Deref<Target = ApmService> + 'static> {
    let guard = state().read();
    if guard.is_some() {
        Some(ApmServiceRef(guard))
    } else {
        None
    }
}

/// Gets the APM session.
pub fn get_session() -> Option<impl core::ops::Deref<Target = ApmSession> + 'static> {
    let guard = state().read();
    if guard.is_some() {
        Some(ApmSessionRef(guard))
    } else {
        None
    }
}

/// Exits the APM service session.
pub fn exit() {
    let mut guard = state().write();
    if let Some(apm_state) = guard.take() {
        // Close in reverse order: session then service
        apm_state.session.close();
        apm_state.service.close();
    }
}

/// Internal storage for APM service and session.
struct ApmState {
    /// Main APM service (IManager)
    service: ApmService,
    /// ISession for performance configuration
    session: ApmSession,
}

/// Wrapper for accessing ApmService through RwLockReadGuard.
struct ApmServiceRef(nx_std_sync::rwlock::RwLockReadGuard<'static, Option<ApmState>>);

impl core::ops::Deref for ApmServiceRef {
    type Target = ApmService;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We only create ApmServiceRef when the option is Some
        &self.0.as_ref().unwrap().service
    }
}

/// Wrapper for accessing ApmSession through RwLockReadGuard.
struct ApmSessionRef(nx_std_sync::rwlock::RwLockReadGuard<'static, Option<ApmState>>);

impl core::ops::Deref for ApmSessionRef {
    type Target = ApmSession;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We only create ApmSessionRef when the option is Some
        &self.0.as_ref().unwrap().session
    }
}

/// Error returned by [`init`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to connect to APM service.
    #[error("failed to connect to APM service")]
    Connect(#[source] nx_service_apm::ConnectError),
    /// Failed to open APM session.
    #[error("failed to open APM session")]
    OpenSession(#[source] nx_service_apm::OpenSessionError),
}
