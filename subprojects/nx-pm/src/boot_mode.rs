//! `pm:bm` (boot mode) service.

pub use nx_service_pm::BootMode;
use nx_service_pm::PmBmService;
use nx_sf::service::DispatchError;

/// Connected `pm:bm` (boot mode) service.
pub struct BootModeService {
    inner: PmBmService,
}

impl BootModeService {
    pub(crate) fn new(inner: PmBmService) -> Self {
        Self { inner }
    }

    /// Returns the current [`BootMode`].
    pub fn get(&self) -> Result<BootMode, BmGetBootModeError> {
        self.inner.get_boot_mode().map_err(BmGetBootModeError)
    }

    /// Sets the boot mode to [`BootMode::Maintenance`].
    pub fn set_maintenance(&self) -> Result<(), BmSetMaintenanceBootError> {
        self.inner
            .set_maintenance_boot()
            .map_err(BmSetMaintenanceBootError)
    }
}

/// IPC dispatch failure from `pm:bm GetBootMode`.
#[derive(Debug, thiserror::Error)]
#[error("pm:bm GetBootMode IPC dispatch failed")]
pub struct BmGetBootModeError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:bm SetMaintenanceBoot`.
#[derive(Debug, thiserror::Error)]
#[error("pm:bm SetMaintenanceBoot IPC dispatch failed")]
pub struct BmSetMaintenanceBootError(#[source] pub DispatchError);
