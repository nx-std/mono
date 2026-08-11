//! APM service state and singleton API.
//!
//! This module manages the APM service session and provides a singleton interface
//! for accessing APM functionality throughout the application lifecycle.

use nx_service_apm::{
    ApmService,
    ApmSession,
};
use nx_std_sync::{
    once_lock::OnceLock,
    rwlock::RwLock,
};

use crate::services::sm;

/// Global APM state, lazily initialized.
static APM_STATE: OnceLock<RwLock<Option<ApmState>>> = OnceLock::new();

/// Returns a reference to the APM state lock, initializing it if needed.
fn state() -> &'static RwLock<Option<ApmState>> {
    APM_STATE.get_or_init(|| RwLock::new(None))
}

/// Opens the performance-management service and its configuration session.
///
/// The session is opened here rather than on first use: every caller needs it,
/// and opening it alongside the service keeps the pair's lifetime single.
///
/// Counts its callers: a second caller joins the session the first opened
/// rather than replacing it, and both close when the last of them calls
/// [`exit`]. Without the count, two independent users of this service in one
/// process would each close it under the other.
///
/// # Errors
///
/// Returns an error when the Service Manager is not open, or when the
/// connection was refused. Nothing was opened.
pub fn init() -> Result<(), ConnectError> {
    {
        let mut guard = state().write();
        if let Some(ref mut apm_state) = *guard {
            apm_state.ref_count += 1;
            return Ok(());
        }
    }

    let sm = sm::session().map_err(ConnectError::SmNotInitialized)?;

    // Connect to APM service
    let service = nx_service_apm::connect(&sm).map_err(ConnectError::Connect)?;

    let session = service.open_session().map_err(ConnectError::OpenSession)?;

    let mut guard = state().write();
    *guard = Some(ApmState {
        service,
        session,
        ref_count: 1,
    });

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
///
/// Decrements the caller count. The session and the service close when it
/// reaches zero.
pub fn exit() {
    let mut guard = state().write();
    if let Some(ref mut apm_state) = *guard {
        apm_state.ref_count = apm_state.ref_count.saturating_sub(1);
        if apm_state.ref_count == 0 {
            // RAII: dropping `ApmState` closes the session and the service in
            // field declaration order, session first.
            let _ = guard.take();
        }
    }
}

/// Internal storage for APM service and session.
struct ApmState {
    /// Main APM service (IManager)
    service: ApmService,
    /// ISession for performance configuration
    session: ApmSession,
    /// How many callers of [`init`] have not yet called [`exit`]
    ref_count: u32,
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
    /// No Service Manager session is open.
    ///
    /// Occurs when the connection is attempted before the Service Manager is
    /// open, or after it is closed. Nothing was opened.
    #[error("the Service Manager is not initialized")]
    SmNotInitialized(#[source] nx_rt_core::services::sm::NotInitializedError),
    /// Failed to connect to APM service.
    #[error("failed to connect to APM service")]
    Connect(#[source] nx_service_apm::ConnectError),
    /// Failed to open APM session.
    #[error("failed to open APM session")]
    OpenSession(#[source] nx_service_apm::OpenSessionError),
}

#[cfg(feature = "ffi")]
impl nx_rt_core::error::ToResultCode for ConnectError {
    fn to_rc(self) -> nx_rt_core::error::ResultCode {
        use nx_sf::error::ToResultCode as _;

        match self {
            Self::SmNotInitialized(err) => err.to_rc(),
            Self::Connect(err) => err.to_rc(),
            Self::OpenSession(err) => err.to_rc(),
        }
    }
}
