//! HID service state and singleton API.
//!
//! This module manages the HID service session and provides a singleton interface
//! for accessing HID functionality throughout the application lifecycle.

use nx_service_hid::HidService;
use nx_std_sync::{once_lock::OnceLock, rwlock::RwLock};

use crate::{applet_manager, service_manager};

/// Global HID state, lazily initialized.
static HID_STATE: OnceLock<RwLock<Option<HidState>>> = OnceLock::new();

/// Returns a reference to the HID state lock, initializing it if needed.
fn state() -> &'static RwLock<Option<HidState>> {
    HID_STATE.get_or_init(|| RwLock::new(None))
}

/// Initializes the HID service.
///
/// This matches libnx's `hidInitialize()` behavior, which connects to the
/// service using the applet resource user ID.
///
/// # Panics
///
/// Panics if SM is not initialized.
pub fn init() -> Result<(), ConnectError> {
    let sm_guard = service_manager::sm_session();
    let sm = sm_guard.as_ref().expect("SM not initialized");

    // Get applet resource user ID from applet manager
    let aruid = applet_manager::get_applet_resource_user_id();

    // Connect to HID service
    let service = nx_service_hid::connect(sm, aruid).map_err(ConnectError::Connect)?;

    let mut guard = state().write();
    *guard = Some(HidState { service });

    Ok(())
}

/// Gets the HID service.
pub fn get_service() -> Option<impl core::ops::Deref<Target = HidService> + 'static> {
    let guard = state().read();
    if guard.is_some() {
        Some(HidServiceRef(guard))
    } else {
        None
    }
}

/// Exits the HID service.
pub fn exit() {
    let mut guard = state().write();
    if let Some(hid_state) = guard.take() {
        hid_state.service.close();
    }
}

/// Internal storage for HID service.
struct HidState {
    /// HID service (IHidServer with IAppletResource and shared memory)
    service: HidService,
}

/// Wrapper for accessing HidService through RwLockReadGuard.
struct HidServiceRef(nx_std_sync::rwlock::RwLockReadGuard<'static, Option<HidState>>);

impl core::ops::Deref for HidServiceRef {
    type Target = HidService;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We only create HidServiceRef when the option is Some
        &self.0.as_ref().unwrap().service
    }
}

/// Error returned by [`init`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to connect to HID service.
    #[error("failed to connect to HID service")]
    Connect(#[source] nx_service_hid::ConnectError),
}
