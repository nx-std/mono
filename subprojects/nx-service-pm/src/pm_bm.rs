//! `pm:bm` (boot mode) service wrapper.

use nx_service_sm::SmService;
use nx_sf::{
    error::{
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
    #[inline]
    pub fn get_boot_mode(&self) -> Result<BootMode, DispatchError> {
        cmif::get_boot_mode(&self.0)
    }

    /// Sets the boot mode to maintenance.
    #[inline]
    pub fn set_maintenance_boot(&self) -> Result<(), DispatchError> {
        cmif::set_maintenance_boot(&self.0)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BootMode {
    Normal = 0,
    Maintenance = 1,
    SafeMode = 2,
}

pub(crate) mod proto {
    use nx_sf::ServiceName;

    /// Service name registered with `sm:`.
    pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("pm:bm");

    /// `GetBootMode` — returns the current [`BootMode`](super::BootMode).
    pub const GET_BOOT_MODE: u32 = 0;
    /// `SetMaintenanceBoot` — switches the boot mode to maintenance.
    pub const SET_MAINTENANCE_BOOT: u32 = 1;
}
