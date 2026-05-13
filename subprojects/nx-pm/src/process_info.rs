//! `pm:info` (process info) service.

pub use nx_service_pm::ResourceLimitValues;
use nx_service_pm::{PmInfoService, ProcessId, ProgramId};
use nx_sf::service::DispatchError;

/// Connected `pm:info` (process info) service.
pub struct ProcessInfoService {
    inner: PmInfoService,
}

impl ProcessInfoService {
    pub(crate) fn new(inner: PmInfoService) -> Self {
        Self { inner }
    }

    /// Resolves a [`ProcessId`] to its [`ProgramId`].
    pub fn program_id(&self, pid: ProcessId) -> Result<ProgramId, InfoGetProgramIdError> {
        self.inner
            .get_program_id(pid)
            .map_err(InfoGetProgramIdError)
    }

    /// Returns the applet's current [`ResourceLimitValues`]
    /// (`[14.0.0+/Atmosphere]`).
    pub fn applet_current_resource_limits(
        &self,
    ) -> Result<ResourceLimitValues, InfoGetAppletCurrentResourceLimitValuesError> {
        self.inner
            .get_applet_current_resource_limit_values()
            .map_err(InfoGetAppletCurrentResourceLimitValuesError)
    }

    /// Returns the applet's peak [`ResourceLimitValues`]
    /// (`[14.0.0+/Atmosphere]`).
    pub fn applet_peak_resource_limits(
        &self,
    ) -> Result<ResourceLimitValues, InfoGetAppletPeakResourceLimitValuesError> {
        self.inner
            .get_applet_peak_resource_limit_values()
            .map_err(InfoGetAppletPeakResourceLimitValuesError)
    }
}

/// IPC dispatch failure from `pm:info GetProgramId`.
#[derive(Debug, thiserror::Error)]
#[error("pm:info GetProgramId IPC dispatch failed")]
pub struct InfoGetProgramIdError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:info AtmosphereGetCurrentAppletResourceUserId`
/// (`GetAppletCurrentResourceLimitValues`).
#[derive(Debug, thiserror::Error)]
#[error("pm:info GetAppletCurrentResourceLimitValues IPC dispatch failed")]
pub struct InfoGetAppletCurrentResourceLimitValuesError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:info GetAppletPeakResourceLimitValues`.
#[derive(Debug, thiserror::Error)]
#[error("pm:info GetAppletPeakResourceLimitValues IPC dispatch failed")]
pub struct InfoGetAppletPeakResourceLimitValuesError(#[source] pub DispatchError);
