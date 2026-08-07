//! `pm:bm` (boot mode) service wrapper.

use nx_service_sm::SmService;
use nx_sf::{
    error::{
        GENERIC_ERROR,
        ResultCode,
        ToResultCode,
    },
    service::{
        DispatchError,
        Session,
    },
};

use super::cmif;

/// Connected `pm:bm` (boot mode) service wrapper.
pub struct PmBmService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for PmBmService {}
unsafe impl Sync for PmBmService {}

impl PmBmService {
    /// Gets the current boot mode.
    ///
    /// # Errors
    ///
    /// Returns [`GetBootModeError::UnknownMode`] when the server answers with
    /// a discriminant [`BootMode`] does not define. libnx casts the reply
    /// straight into `PmBootMode` and hands it to the caller; doing that in
    /// Rust would build an enum out of a value it has no variant for, which is
    /// undefined behaviour, so the value is checked instead.
    #[inline]
    pub fn get_boot_mode(&self) -> Result<BootMode, GetBootModeError> {
        let raw = cmif::get_boot_mode(&self.0).map_err(GetBootModeError::Dispatch)?;

        raw.try_into().map_err(GetBootModeError::UnknownMode)
    }

    /// Sets the boot mode to maintenance.
    #[inline]
    pub fn set_maintenance_boot(&self) -> Result<(), DispatchError> {
        cmif::set_maintenance_boot(&self.0)
    }
}

/// Errors returned by [`PmBmService::get_boot_mode`].
#[derive(Debug, thiserror::Error)]
pub enum GetBootModeError {
    /// The IPC dispatch failed, so no reply was decoded.
    #[error("failed to dispatch pm:bm GetBootMode")]
    Dispatch(#[source] DispatchError),

    /// The server replied successfully with a boot mode this crate does not
    /// define.
    ///
    /// Occurs when firmware gains a boot mode newer than [`BootMode`]. The
    /// session is unaffected and the call is safe to retry, though a retry
    /// returns the same unknown value until [`BootMode`] learns the variant.
    #[error("pm:bm GetBootMode replied with an unknown boot mode")]
    UnknownMode(#[source] UnknownBootMode),
}

impl ToResultCode for GetBootModeError {
    fn to_rc(self) -> ResultCode {
        match self {
            GetBootModeError::Dispatch(err) => err.to_rc(),
            // The server answered without error, so it assigned this failure no
            // code of its own; it is a local decode failure like any other.
            GetBootModeError::UnknownMode(_) => GENERIC_ERROR,
        }
    }
}

#[cfg(feature = "ffi")]
impl PmBmService {
    /// Returns the underlying session for libnx `Service*` shadow buffers.
    #[inline]
    pub fn session(&self) -> &Session {
        &self.0
    }
}

/// Connects to the `pm:bm` (boot mode) service using CMIF.
pub fn connect_bm_cmif(sm: &SmService) -> Result<PmBmService, ConnectBmCmifError> {
    let handle = sm
        .get_service_handle_cmif(proto::SERVICE_NAME)
        .map_err(ConnectBmCmifError)?;

    let service = Session::new(handle, 0);

    Ok(PmBmService(service))
}

/// Error returned by [`connect_bm_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get pm:bm service")]
pub struct ConnectBmCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

impl ToResultCode for ConnectBmCmifError {
    fn to_rc(self) -> ResultCode {
        self.0.to_rc()
    }
}

/// Boot mode returned by `pm:bm` `GetBootMode`.
///
/// The wire form is a bare `u32`, and only the three discriminants below are
/// inhabited, so a reply is decoded as a `u32` and converted through
/// [`TryFrom`] rather than read directly into this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BootMode {
    Normal = 0,
    Maintenance = 1,
    SafeMode = 2,
}

impl TryFrom<u32> for BootMode {
    type Error = UnknownBootMode;

    fn try_from(raw: u32) -> Result<Self, Self::Error> {
        match raw {
            0 => Ok(BootMode::Normal),
            1 => Ok(BootMode::Maintenance),
            2 => Ok(BootMode::SafeMode),
            _ => Err(UnknownBootMode(raw)),
        }
    }
}

/// Error returned when a `u32` names no [`BootMode`] variant.
#[derive(Debug, thiserror::Error)]
#[error("{0} is not a known pm:bm boot mode")]
pub struct UnknownBootMode(pub u32);

pub(crate) mod proto {
    use nx_sf::ServiceName;

    /// Service name registered with `sm:`.
    pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("pm:bm");

    /// `GetBootMode` — returns the current [`BootMode`](super::BootMode).
    pub const GET_BOOT_MODE: u32 = 0;
    /// `SetMaintenanceBoot` — switches the boot mode to maintenance.
    pub const SET_MAINTENANCE_BOOT: u32 = 1;
}
