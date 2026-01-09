//! System Settings Service (set:sys) Implementation.
//!
//! This crate provides access to the Nintendo Switch's system settings service,
//! which allows querying firmware version and other system configuration.
//!
//! ## Protocol Support
//!
//! The service supports two protocols:
//! - **CMIF**: Available on all HOS versions
//! - **TIPC**: Available on HOS 12.0.0+ and Atmosphere
//!
//! Protocol selection is the caller's responsibility. Use the `_cmif` or `_tipc`
//! method variants as appropriate for your system version.

#![no_std]

extern crate nx_panic_handler; // Provide #![panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::Service;
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod proto;
mod tipc;

pub use self::{
    cmif::GetFirmwareVersionError as GetFirmwareVersionCmifError,
    proto::{FirmwareVersion, SERVICE_NAME},
    tipc::GetFirmwareVersionError as GetFirmwareVersionTipcError,
};

/// System Settings Service (set:sys) session wrapper.
///
/// Provides type safety to distinguish set:sys sessions from other services.
#[repr(transparent)]
pub struct SetSysService(Service);

impl SetSysService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.session
    }

    /// Consumes and closes the set:sys session.
    #[inline]
    pub fn close(self) {
        self.0.close();
    }
}

/// CMIF protocol methods.
impl SetSysService {
    /// Gets the system firmware version using CMIF protocol.
    ///
    /// Uses command ID 4 (GetFirmwareVersion2) which is available on HOS 3.0.0+.
    #[inline]
    pub fn get_firmware_version_cmif(
        &self,
    ) -> Result<FirmwareVersion, GetFirmwareVersionCmifError> {
        cmif::get_firmware_version(self.0.session)
    }

    /// Gets the system firmware version using CMIF protocol (legacy command).
    ///
    /// Uses command ID 3 (GetFirmwareVersion) for pre-3.0.0 systems.
    /// This command zeros the revision field in the output.
    #[inline]
    pub fn get_firmware_version_legacy_cmif(
        &self,
    ) -> Result<FirmwareVersion, GetFirmwareVersionCmifError> {
        cmif::get_firmware_version_legacy(self.0.session)
    }
}

/// TIPC protocol methods.
///
/// Requires HOS 12.0.0+ or Atmosphere.
impl SetSysService {
    /// Gets the system firmware version using TIPC protocol.
    ///
    /// Uses command ID 4 (GetFirmwareVersion2).
    /// Requires HOS 12.0.0+ or Atmosphere.
    #[inline]
    pub fn get_firmware_version_tipc(
        &self,
    ) -> Result<FirmwareVersion, GetFirmwareVersionTipcError> {
        tipc::get_firmware_version(self.0.session)
    }

    /// Gets the system firmware version using TIPC protocol (legacy command).
    ///
    /// Uses command ID 3 (GetFirmwareVersion).
    /// This command zeros the revision field in the output.
    #[inline]
    pub fn get_firmware_version_legacy_tipc(
        &self,
    ) -> Result<FirmwareVersion, GetFirmwareVersionTipcError> {
        tipc::get_firmware_version_legacy(self.0.session)
    }
}

/// Connects to the set:sys (System Settings) service using CMIF.
///
/// Obtains a service handle from the Service Manager using CMIF protocol.
/// For TIPC-based systems (HOS 12.0.0+), use [`connect_tipc`].
pub fn connect_cmif(sm: &SmService) -> Result<SetSysService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Service {
        session: handle,
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };

    Ok(SetSysService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get set:sys service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

/// Connects to the set:sys (System Settings) service using TIPC.
///
/// Obtains a service handle from the Service Manager using TIPC protocol.
/// Requires HOS 12.0.0+ or Atmosphere.
pub fn connect_tipc(sm: &SmService) -> Result<SetSysService, ConnectTipcError> {
    let handle = sm
        .get_service_handle_tipc(SERVICE_NAME)
        .map_err(ConnectTipcError)?;

    let service = Service {
        session: handle,
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };

    Ok(SetSysService(service))
}

/// Error returned by [`connect_tipc`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get set:sys service")]
pub struct ConnectTipcError(#[source] pub nx_service_sm::GetServiceTipcError);
