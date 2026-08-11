//! HID service state and singleton API.
//!
//! This module manages the HID service session and provides a singleton interface
//! for accessing HID functionality throughout the application lifecycle.

use nx_service_hid::HidService;
use nx_std_sync::{
    once_lock::OnceLock,
    rwlock::RwLock,
};

use crate::services::{
    applet,
    sm,
};

/// Global HID state, lazily initialized.
static HID_STATE: OnceLock<RwLock<Option<HidState>>> = OnceLock::new();

/// Returns a reference to the HID state lock, initializing it if needed.
fn state() -> &'static RwLock<Option<HidState>> {
    HID_STATE.get_or_init(|| RwLock::new(None))
}

/// Opens the input service for this process.
///
/// The session is bound to the applet resource user id, which is what ties the
/// input the process receives to the role it was launched in.
///
/// Counts its callers: a second caller joins the session the first opened
/// rather than replacing it, and it closes when the last of them calls
/// [`exit`]. Without the count, two independent users of this service in one
/// process would each close it under the other, leaving the survivor reading
/// shared memory that has already been unmapped.
///
/// # Errors
///
/// Returns an error when the Service Manager is not open, or when the
/// connection was refused. Nothing was opened.
pub fn init() -> Result<(), ConnectError> {
    {
        let mut guard = state().write();
        if let Some(ref mut hid_state) = *guard {
            hid_state.ref_count += 1;
            return Ok(());
        }
    }

    let sm = sm::session().map_err(ConnectError::SmNotInitialized)?;

    // Get applet resource user ID from applet manager
    let aruid = applet::get_applet_resource_user_id();

    // Connect to HID service
    let service = nx_service_hid::connect(&sm, aruid).map_err(ConnectError::Connect)?;

    let mut guard = state().write();
    *guard = Some(HidState {
        service,
        ref_count: 1,
    });

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
///
/// Decrements the caller count. The session closes when it reaches zero.
pub fn exit() {
    let mut guard = state().write();
    if let Some(ref mut hid_state) = *guard {
        hid_state.ref_count = hid_state.ref_count.saturating_sub(1);
        if hid_state.ref_count == 0 {
            // `HidService` is RAII; dropping the taken state closes both
            // kernel handles (main session + IAppletResource) and unmaps the
            // shared memory.
            let _ = guard.take();
        }
    }
}

/// Internal storage for HID service.
struct HidState {
    /// HID service (IHidServer with IAppletResource and shared memory)
    service: HidService,
    /// How many callers of [`init`] have not yet called [`exit`]
    ref_count: u32,
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
    /// No Service Manager session is open.
    ///
    /// Occurs when the connection is attempted before the Service Manager is
    /// open, or after it is closed. Nothing was opened.
    #[error("the Service Manager is not initialized")]
    SmNotInitialized(#[source] nx_rt_core::services::sm::NotInitializedError),
    /// The HID service refused the connection.
    ///
    /// Occurs when the server was unreachable or rejected the request. Nothing
    /// was opened.
    #[error("failed to connect to the HID service")]
    Connect(#[source] nx_service_hid::ConnectError),
}

#[cfg(feature = "ffi")]
impl nx_rt_core::error::ToResultCode for ConnectError {
    fn to_rc(self) -> nx_rt_core::error::ResultCode {
        use nx_sf::error::ToResultCode as _;

        match self {
            Self::SmNotInitialized(err) => err.to_rc(),
            Self::Connect(err) => err.to_rc(),
        }
    }
}
