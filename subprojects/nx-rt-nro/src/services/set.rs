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

/// Reads the system firmware version.
///
/// Picks the protocol the way [`init`] did, then picks the command the way
/// libnx's `setsysGetFirmwareVersion` does: `GetFirmwareVersion2` from HOS
/// 3.0.0, and `GetFirmwareVersion` before it. The two differ only in that the
/// older one zeroes the revision field.
///
/// The startup version resolution calls this with no version published yet,
/// which reads as "older than 3.0.0" and selects the legacy command. That is
/// libnx's behaviour too, and it is the safe direction: the legacy command
/// exists on every firmware, and its answer is what publishes the version the
/// later calls then select on.
///
/// # Errors
///
/// Returns [`FirmwareVersionError::NotInitialized`] when no session is open,
/// and the protocol's own error when the command failed.
pub fn firmware_version() -> Result<nx_service_set::FirmwareVersion, FirmwareVersionError> {
    let service = get_service().ok_or(FirmwareVersionError::NotInitialized)?;
    let legacy = nx_rt_core::env::hos_version::get()
        < nx_rt_core::env::hos_version::HosVersion::new(3, 0, 0);

    match (sm::should_use_tipc(), legacy) {
        (true, true) => service
            .get_firmware_version_legacy_tipc()
            .map_err(FirmwareVersionError::Tipc),
        (true, false) => service
            .get_firmware_version_tipc()
            .map_err(FirmwareVersionError::Tipc),
        (false, true) => service
            .get_firmware_version_legacy_cmif()
            .map_err(FirmwareVersionError::Cmif),
        (false, false) => service
            .get_firmware_version_cmif()
            .map_err(FirmwareVersionError::Cmif),
    }
}

/// Error returned by [`firmware_version`].
#[derive(Debug, thiserror::Error)]
pub enum FirmwareVersionError {
    /// No `set:sys` session is open.
    ///
    /// Occurs when the version is read before [`init`] has connected. Nothing
    /// was sent.
    #[error("the set:sys service is not initialized")]
    NotInitialized,
    /// The command failed over CMIF.
    ///
    /// Occurs when the server refused the request or the reply could not be
    /// decoded, on a session opened with the CMIF protocol. Nothing was
    /// published; the version the caller was resolving is left as it was.
    #[error("failed to read the firmware version (CMIF)")]
    Cmif(#[source] nx_service_set::GetFirmwareVersionCmifError),

    /// The command failed over TIPC.
    ///
    /// The same as [`FirmwareVersionError::Cmif`], for a session opened with
    /// the TIPC protocol.
    #[error("failed to read the firmware version (TIPC)")]
    Tipc(#[source] nx_service_set::GetFirmwareVersionTipcError),
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
