//! The system settings interface (`set:sys`): the firmware, the theme, and the settings items a
//! console is configured by.
//!
//! ## Two protocols
//!
//! The firmware version is reachable over CMIF and over TIPC, and the caller picks. Every other
//! command here is CMIF: the settings-item pair addresses an item through two pointer buffers,
//! and this workspace's TIPC codec encodes mapped buffers only. Which protocol the Service
//! Manager was asked with does not decide this; a session carries whatever its commands are sent
//! as.
//!
//! ## Reading a settings item
//!
//! An item is addressed by a section name and a key, and its width is a property of the item
//! rather than of the command, so reading one is two commands: ask how wide it is, then read it
//! into a buffer of that width. A caller that already knows the width, as one reading a `u64`
//! flag does, may read it directly and check the width the interface reports back.

use nx_service_sm::SmService;
use nx_sf::service::{
    BorrowedSessionHandle,
    DispatchError,
    Session,
};

mod cmif;
mod proto;
mod tipc;

pub use self::{
    cmif::GetFirmwareVersionError as GetFirmwareVersionCmifError,
    proto::{
        ColorSetId,
        FirmwareVersion,
        InvalidSettingsText,
        SERVICE_NAME,
        SettingsItemKey,
        SettingsName,
        UnknownColorSetId,
    },
    tipc::GetFirmwareVersionError as GetFirmwareVersionTipcError,
};

/// A connected session to the system settings interface.
#[repr(transparent)]
pub struct SetSysService(Session);

impl SetSysService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl SetSysService {
    /// Gets the system firmware version using CMIF protocol.
    ///
    /// Uses command ID 4 (GetFirmwareVersion2) which is available on HOS 3.0.0+.
    ///
    /// # Errors
    ///
    /// Returns [`GetFirmwareVersionCmifError`] when the request could not be sent or the reply
    /// could not be decoded. Nothing is read.
    #[inline]
    pub fn get_firmware_version_cmif(
        &self,
    ) -> Result<FirmwareVersion, GetFirmwareVersionCmifError> {
        cmif::get_firmware_version(self.0.handle())
    }

    /// Gets the system firmware version using CMIF protocol (legacy command).
    ///
    /// Uses command ID 3 (GetFirmwareVersion) for pre-3.0.0 systems.
    /// This command zeros the revision field in the output.
    ///
    /// # Errors
    ///
    /// The same as [`get_firmware_version_cmif`](Self::get_firmware_version_cmif).
    #[inline]
    pub fn get_firmware_version_legacy_cmif(
        &self,
    ) -> Result<FirmwareVersion, GetFirmwareVersionCmifError> {
        cmif::get_firmware_version_legacy(self.0.handle())
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
    ///
    /// # Errors
    ///
    /// Returns [`GetFirmwareVersionTipcError`] when the request could not be sent or the reply
    /// could not be decoded. Nothing is read.
    #[inline]
    pub fn get_firmware_version_tipc(
        &self,
    ) -> Result<FirmwareVersion, GetFirmwareVersionTipcError> {
        tipc::get_firmware_version(self.0.handle())
    }

    /// Gets the system firmware version using TIPC protocol (legacy command).
    ///
    /// Uses command ID 3 (GetFirmwareVersion).
    /// This command zeros the revision field in the output.
    ///
    /// # Errors
    ///
    /// The same as [`get_firmware_version_tipc`](Self::get_firmware_version_tipc).
    #[inline]
    pub fn get_firmware_version_legacy_tipc(
        &self,
    ) -> Result<FirmwareVersion, GetFirmwareVersionTipcError> {
        tipc::get_firmware_version_legacy(self.0.handle())
    }
}

impl SetSysService {
    /// Reads which of the two system themes the console is set to.
    ///
    /// # Errors
    ///
    /// Returns [`GetColorSetIdError::Dispatch`] when the command failed, and
    /// [`GetColorSetIdError::UnknownTheme`] when the console answered with a theme this crate
    /// does not know.
    pub fn get_color_set_id(&self) -> Result<ColorSetId, GetColorSetIdError> {
        let raw = cmif::get_color_set_id(&self.0).map_err(GetColorSetIdError::Dispatch)?;

        ColorSetId::try_from(raw).map_err(GetColorSetIdError::UnknownTheme)
    }

    /// Reads how many bytes the settings item `key` names inside the `name` section takes.
    ///
    /// # Errors
    ///
    /// Returns [`DispatchError`] when the command failed, which is the answer for an item the
    /// console does not hold. Nothing is read.
    pub fn get_settings_item_value_size(
        &self,
        name: &SettingsName,
        key: &SettingsItemKey,
    ) -> Result<u64, DispatchError> {
        cmif::get_settings_item_value_size(&self.0, name, key)
    }

    /// Reads the settings item `key` names inside the `name` section into `value`.
    ///
    /// Returns how many bytes the interface wrote, which is the item's own width and not
    /// necessarily the width of `value`.
    ///
    /// # Errors
    ///
    /// The same as [`get_settings_item_value_size`](Self::get_settings_item_value_size).
    pub fn get_settings_item_value(
        &self,
        name: &SettingsName,
        key: &SettingsItemKey,
        value: &mut [u8],
    ) -> Result<u64, DispatchError> {
        cmif::get_settings_item_value(&self.0, name, key, value)
    }

    /// Reads the settings item `key` names inside the `name` section as a `u64`.
    ///
    /// A settings item's width belongs to the item, so this refuses one that is not eight bytes
    /// wide rather than reading part of it: a caller asking for a number wants the whole number.
    ///
    /// # Errors
    ///
    /// Returns [`GetSettingsItemValueU64Error::Dispatch`] when the command failed, which is the
    /// answer for an item the console does not hold, and
    /// [`GetSettingsItemValueU64Error::UnexpectedWidth`] when the item is not eight bytes wide.
    pub fn get_settings_item_value_u64(
        &self,
        name: &SettingsName,
        key: &SettingsItemKey,
    ) -> Result<u64, GetSettingsItemValueU64Error> {
        let mut value = [0u8; size_of::<u64>()];
        let written = cmif::get_settings_item_value(&self.0, name, key, &mut value)
            .map_err(GetSettingsItemValueU64Error::Dispatch)?;

        if written != value.len() as u64 {
            return Err(GetSettingsItemValueU64Error::UnexpectedWidth { width: written });
        }

        Ok(u64::from_le_bytes(value))
    }
}

/// Error returned by [`SetSysService::get_color_set_id`].
#[derive(Debug, thiserror::Error)]
pub enum GetColorSetIdError {
    /// The command failed
    ///
    /// Occurs when the request could not be sent, or the reply could not be decoded. Nothing was
    /// read.
    #[error("failed to read the selected system theme")]
    Dispatch(#[source] DispatchError),

    /// The answer names no theme this crate knows
    ///
    /// Occurs when the console answers with a theme added after this crate's list was written.
    #[error("the console answered with a system theme this crate does not know")]
    UnknownTheme(#[source] UnknownColorSetId),
}

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for GetColorSetIdError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Dispatch(err) => err.to_rc(),
            Self::UnknownTheme(_) => nx_sf::error::GENERIC_ERROR,
        }
    }
}

/// Error returned by [`SetSysService::get_settings_item_value_u64`].
#[derive(Debug, thiserror::Error)]
pub enum GetSettingsItemValueU64Error {
    /// The command failed
    ///
    /// Occurs when the request could not be sent, when the console holds no such item, or when
    /// the reply could not be decoded. Nothing was read.
    #[error("failed to read the settings item")]
    Dispatch(#[source] DispatchError),

    /// The item is not the width of the value asked for
    ///
    /// Occurs when the item exists but is not eight bytes wide. The bytes the interface did write
    /// are discarded, because a number assembled from part of an item is not that item's value.
    #[error("the settings item is {width} bytes wide, not eight")]
    UnexpectedWidth {
        /// How wide the interface reported the item to be.
        width: u64,
    },
}

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for GetSettingsItemValueU64Error {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Dispatch(err) => err.to_rc(),
            Self::UnexpectedWidth { .. } => nx_sf::error::GENERIC_ERROR,
        }
    }
}

/// Opens a session to the system settings interface, asking the Service Manager over CMIF.
///
/// For TIPC-based systems (HOS 12.0.0+), use [`connect_sys_tipc`].
///
/// # Errors
///
/// Returns [`ConnectSysCmifError`] when the Service Manager refused to hand out the interface.
/// Nothing was opened.
pub fn connect_sys_cmif(sm: &SmService) -> Result<SetSysService, ConnectSysCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectSysCmifError)?;

    Ok(SetSysService(Session::new(handle, 0)))
}

/// Error returned by [`connect_sys_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get set:sys service")]
pub struct ConnectSysCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for ConnectSysCmifError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        self.0.to_rc()
    }
}

/// Opens a session to the system settings interface, asking the Service Manager over TIPC.
///
/// Requires HOS 12.0.0+ or Atmosphere.
///
/// # Errors
///
/// Returns [`ConnectSysTipcError`] when the Service Manager refused to hand out the interface.
/// Nothing was opened.
pub fn connect_sys_tipc(sm: &SmService) -> Result<SetSysService, ConnectSysTipcError> {
    let handle = sm
        .get_service_handle_tipc(SERVICE_NAME)
        .map_err(ConnectSysTipcError)?;

    Ok(SetSysService(Session::new(handle, 0)))
}

/// Error returned by [`connect_sys_tipc`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get set:sys service")]
pub struct ConnectSysTipcError(#[source] pub nx_service_sm::GetServiceTipcError);

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for ConnectSysTipcError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        self.0.to_rc()
    }
}
