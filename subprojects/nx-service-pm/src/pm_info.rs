//! `pm:info` (process info) service wrapper.

use core::mem::size_of;

use nx_service_sm::SmService;
use nx_sf::{
    error::{ResultCode, ToResultCode},
    service::{DispatchError, Session},
};
use static_assertions::const_assert_eq;

use super::{
    cmif,
    types::{ProcessId, ProgramId},
};

/// Connected `pm:info` (process info) service wrapper.
pub struct PmInfoService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for PmInfoService {}
unsafe impl Sync for PmInfoService {}

impl PmInfoService {
    /// Gets a program ID from a process ID.
    #[inline]
    pub fn get_program_id(&self, pid: ProcessId) -> Result<ProgramId, DispatchError> {
        cmif::info_get_program_id(&self.0, pid)
    }

    /// Gets the applet's current resource limit values.
    ///
    /// `[14.0.0+/Atmosphere]`
    #[inline]
    pub fn get_applet_current_resource_limit_values(
        &self,
    ) -> Result<ResourceLimitValues, DispatchError> {
        cmif::get_applet_current_resource_limit_values(&self.0)
    }

    /// Gets the applet's peak resource limit values.
    ///
    /// `[14.0.0+/Atmosphere]`
    #[inline]
    pub fn get_applet_peak_resource_limit_values(
        &self,
    ) -> Result<ResourceLimitValues, DispatchError> {
        cmif::get_applet_peak_resource_limit_values(&self.0)
    }
}

#[cfg(feature = "ffi")]
impl PmInfoService {
    /// Returns the underlying session for libnx `Service*` shadow buffers.
    #[inline]
    pub fn session(&self) -> &Session {
        &self.0
    }
}

/// Connects to the `pm:info` (process info) service using CMIF.
pub fn connect_info_cmif(sm: &SmService) -> Result<PmInfoService, ConnectInfoCmifError> {
    let handle = sm
        .get_service_handle_cmif(proto::SERVICE_NAME)
        .map_err(ConnectInfoCmifError)?;

    let service = Session::new(handle, 0);

    Ok(PmInfoService(service))
}

/// Error returned by [`connect_info_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get pm:info service")]
pub struct ConnectInfoCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

impl ToResultCode for ConnectInfoCmifError {
    fn to_rc(self) -> ResultCode {
        self.0.to_rc()
    }
}

/// Resource limit values returned by `pm:info` `GetApplet*ResourceLimitValues`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ResourceLimitValues {
    pub physical_memory: u64,
    pub thread_count: u32,
    pub event_count: u32,
    pub transfer_memory_count: u32,
    pub session_count: u32,
}

const_assert_eq!(size_of::<ResourceLimitValues>(), 0x18);

pub(crate) mod proto {
    use nx_sf::ServiceName;

    /// Service name registered with `sm:`.
    pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("pm:info");

    /// `GetProgramId` — resolves a process ID to its program ID.
    pub const GET_PROGRAM_ID: u32 = 0;
    /// `GetAppletCurrentResourceLimitValues`.
    ///
    /// `[14.0.0+/Atmosphere]`
    pub const GET_APPLET_CURRENT_RESOURCE_LIMIT_VALUES: u32 = 1;
    /// `GetAppletPeakResourceLimitValues`.
    ///
    /// `[14.0.0+/Atmosphere]`
    pub const GET_APPLET_PEAK_RESOURCE_LIMIT_VALUES: u32 = 2;
}
