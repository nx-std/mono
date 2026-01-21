//! VI service state and singleton API.
//!
//! This module manages the VI service session and provides a singleton interface
//! for accessing display and layer functionality throughout the application lifecycle.

use nx_service_vi::{ViService, types::ViServiceType};
use nx_std_sync::{once_lock::OnceLock, rwlock::RwLock};

use crate::service_manager;

/// Global VI state, lazily initialized.
static VI_STATE: OnceLock<RwLock<Option<ViState>>> = OnceLock::new();

/// Returns a reference to the VI state lock, initializing it if needed.
fn state() -> &'static RwLock<Option<ViState>> {
    VI_STATE.get_or_init(|| RwLock::new(None))
}

/// Initializes the VI service with the given service type.
///
/// This matches libnx's `viInitialize()` behavior with reference counting.
/// Multiple calls increment the reference count; actual initialization only
/// happens on the first call.
///
/// # Panics
///
/// Panics if SM is not initialized.
pub fn init(service_type: ViServiceType) -> Result<(), ConnectError> {
    let mut guard = state().write();

    // If already initialized, just increment ref count
    if let Some(ref mut vi_state) = *guard {
        vi_state.ref_count += 1;
        return Ok(());
    }

    let sm_guard = service_manager::sm_session();
    let sm = sm_guard.as_ref().expect("SM not initialized");

    // Connect to VI service
    let service = nx_service_vi::connect(sm, service_type).map_err(ConnectError::Connect)?;

    *guard = Some(ViState {
        service,
        ref_count: 1,
    });

    Ok(())
}

/// Initializes the VI service with default configuration.
///
/// Uses auto service type detection (tries Manager, then System, then Application).
pub fn init_default() -> Result<(), ConnectError> {
    init(ViServiceType::Default)
}

/// Gets the VI service.
pub fn get_service() -> Option<impl core::ops::Deref<Target = ViService> + 'static> {
    let guard = state().read();
    if guard.is_some() {
        Some(ViServiceRef(guard))
    } else {
        None
    }
}

/// Exits the VI service.
///
/// Decrements the reference count. Actual cleanup only happens when the
/// reference count reaches 0.
pub fn exit() {
    let mut guard = state().write();
    if let Some(ref mut vi_state) = *guard {
        vi_state.ref_count = vi_state.ref_count.saturating_sub(1);
        if vi_state.ref_count == 0 {
            // Take and drop the service (closes on Drop)
            guard.take();
        }
    }
}

/// Returns true if the VI service is currently initialized.
pub fn is_initialized() -> bool {
    state().read().is_some()
}

/// Internal storage for VI service.
struct ViState {
    /// VI service session
    service: ViService,
    /// Reference count for service guard pattern (like libnx's NX_GENERATE_SERVICE_GUARD)
    ref_count: u32,
}

/// Wrapper for accessing ViService through RwLockReadGuard.
struct ViServiceRef(nx_std_sync::rwlock::RwLockReadGuard<'static, Option<ViState>>);

impl core::ops::Deref for ViServiceRef {
    type Target = ViService;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We only create ViServiceRef when the option is Some
        &self.0.as_ref().unwrap().service
    }
}

/// Error returned by [`init`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to connect to VI service.
    #[error("failed to connect to VI service")]
    Connect(#[source] nx_service_vi::ConnectError),
}

/// Global configuration storage for VI service type.
///
/// Mirrors the weak symbol pattern from libnx:
/// - `__nx_vi_service_type`
static VI_CONFIG: OnceLock<RwLock<ViConfigState>> = OnceLock::new();

struct ViConfigState {
    service_type: ViServiceType,
}

impl Default for ViConfigState {
    fn default() -> Self {
        Self {
            service_type: ViServiceType::Default,
        }
    }
}

/// Gets the current VI service type configuration.
pub fn get_service_type() -> ViServiceType {
    VI_CONFIG
        .get_or_init(|| RwLock::new(ViConfigState::default()))
        .read()
        .service_type
}

/// Sets the VI service type configuration.
///
/// Must be called before `init()` to have effect.
pub fn set_service_type(service_type: ViServiceType) {
    VI_CONFIG
        .get_or_init(|| RwLock::new(ViConfigState::default()))
        .write()
        .service_type = service_type;
}

/// Creates configuration and initializes using the global service type setting.
pub fn init_with_config() -> Result<(), ConnectError> {
    init(get_service_type())
}
