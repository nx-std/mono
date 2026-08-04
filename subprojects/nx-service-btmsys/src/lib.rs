//! Bluetooth Manager System service (`btm:sys`) implementation.
//!
//! Provides gamepad pairing, radio control, and audio device management
//! for the Nintendo Switch. Audio device commands are only available on
//! \[13.0.0+\]; hosversion gating is left to the caller per IC-4.
//!
//! ## Usage
//!
//! 1. Connect to the service via [`connect_cmif`].
//! 2. Call gamepad, radio, or audio device methods on [`BtmsysService`].
//! 3. The session is closed automatically on `Drop`.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{
    BorrowedSessionHandle,
    OwnedSessionHandle,
    Session,
};
use nx_svc::ipc::Handle;

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::{
        AcquireEventError,
        AcquireEventWithFlagError,
    },
    proto::SERVICE_NAME,
    types::{
        BtdrvAddress,
        BtmAudioDevice,
    },
};

/// Bluetooth Manager System service wrapper (IBtmSystemCore).
pub struct BtmsysService(Session);

impl BtmsysService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

// ---------------------------------------------------------------------------
// Gamepad pairing commands
// ---------------------------------------------------------------------------

impl BtmsysService {
    /// Starts gamepad pairing (cmd 0).
    #[inline]
    pub fn start_gamepad_pairing(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::start_gamepad_pairing(&self.0)
    }

    /// Cancels gamepad pairing (cmd 1).
    #[inline]
    pub fn cancel_gamepad_pairing(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::cancel_gamepad_pairing(&self.0)
    }

    /// Clears the gamepad pairing database (cmd 2).
    #[inline]
    pub fn clear_gamepad_pairing_database(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::clear_gamepad_pairing_database(&self.0)
    }

    /// Gets the paired gamepad count (cmd 3).
    #[inline]
    pub fn get_paired_gamepad_count(&self) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_paired_gamepad_count(&self.0)
    }

    /// Acquires the gamepad pairing event (cmd 8, 3.0.0+).
    ///
    /// Returns a copy handle for the event (autoclear=true).
    #[inline]
    pub fn acquire_gamepad_pairing_event(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_gamepad_pairing_event(&self.0)
    }

    /// Returns whether gamepad pairing is currently started (cmd 9, 3.0.0+).
    #[inline]
    pub fn is_gamepad_pairing_started(&self) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_gamepad_pairing_started(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Radio commands
// ---------------------------------------------------------------------------

impl BtmsysService {
    /// Enables Bluetooth radio (cmd 4).
    #[inline]
    pub fn enable_radio(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::enable_radio(&self.0)
    }

    /// Disables Bluetooth radio (cmd 5).
    #[inline]
    pub fn disable_radio(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::disable_radio(&self.0)
    }

    /// Returns whether the Bluetooth radio is on (cmd 6).
    #[inline]
    pub fn get_radio_on_off(&self) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::get_radio_on_off(&self.0)
    }

    /// Acquires the radio event (cmd 7, 3.0.0+).
    ///
    /// Returns a copy handle for the event (autoclear=true).
    #[inline]
    pub fn acquire_radio_event(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_radio_event(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Audio device commands (13.0.0+)
// ---------------------------------------------------------------------------

impl BtmsysService {
    /// Starts audio device discovery (cmd 10, 13.0.0+).
    #[inline]
    pub fn start_audio_device_discovery(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::start_audio_device_discovery(&self.0)
    }

    /// Stops audio device discovery (cmd 11, 13.0.0+).
    #[inline]
    pub fn stop_audio_device_discovery(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::stop_audio_device_discovery(&self.0)
    }

    /// Returns whether audio device discovery is in progress (cmd 12, 13.0.0+).
    #[inline]
    pub fn is_discoverying_audio_device(&self) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_discoverying_audio_device(&self.0)
    }

    /// Gets discovered audio devices (cmd 13, 13.0.0+).
    ///
    /// Writes results into the caller's buffer and returns the count written.
    #[inline]
    pub fn get_discovered_audio_device(
        &self,
        out: &mut [BtmAudioDevice],
    ) -> Result<i32, nx_sf::service::DispatchError> {
        cmif::get_discovered_audio_device(&self.0, out)
    }

    /// Acquires the audio device connection event (cmd 14, 13.0.0+).
    ///
    /// Returns a copy handle for the event (autoclear=true).
    #[inline]
    pub fn acquire_audio_device_connection_event(&self) -> Result<u32, AcquireEventError> {
        cmif::acquire_audio_device_connection_event(&self.0)
    }

    /// Connects to an audio device by address (cmd 15, 13.0.0+).
    #[inline]
    pub fn connect_audio_device(
        &self,
        addr: &BtdrvAddress,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::connect_audio_device(&self.0, addr)
    }

    /// Returns whether an audio device connection is in progress (cmd 16, 13.0.0+).
    #[inline]
    pub fn is_connecting_audio_device(&self) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_connecting_audio_device(&self.0)
    }

    /// Gets connected audio devices (cmd 17, 13.0.0+).
    ///
    /// Writes results into the caller's buffer and returns the count written.
    #[inline]
    pub fn get_connected_audio_devices(
        &self,
        out: &mut [BtmAudioDevice],
    ) -> Result<i32, nx_sf::service::DispatchError> {
        cmif::get_connected_audio_devices(&self.0, out)
    }

    /// Disconnects an audio device by address (cmd 18, 13.0.0+).
    #[inline]
    pub fn disconnect_audio_device(
        &self,
        addr: &BtdrvAddress,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::disconnect_audio_device(&self.0, addr)
    }

    /// Acquires the paired audio device info changed event (cmd 19, 13.0.0+).
    ///
    /// Returns a copy handle for the event (autoclear=true).
    #[inline]
    pub fn acquire_paired_audio_device_info_changed_event(&self) -> Result<u32, AcquireEventError> {
        cmif::acquire_paired_audio_device_info_changed_event(&self.0)
    }

    /// Gets paired audio devices (cmd 20, 13.0.0+).
    ///
    /// Writes results into the caller's buffer and returns the count written.
    #[inline]
    pub fn get_paired_audio_devices(
        &self,
        out: &mut [BtmAudioDevice],
    ) -> Result<i32, nx_sf::service::DispatchError> {
        cmif::get_paired_audio_devices(&self.0, out)
    }

    /// Removes audio device pairing by address (cmd 21, 13.0.0+).
    #[inline]
    pub fn remove_audio_device_pairing(
        &self,
        addr: &BtdrvAddress,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::remove_audio_device_pairing(&self.0, addr)
    }

    /// Requests audio device connection rejection (cmd 22, 13.0.0+).
    ///
    /// Sends PID and the applet resource user ID.
    #[inline]
    pub fn request_audio_device_connection_rejection(
        &self,
        applet_resource_user_id: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::request_audio_device_connection_rejection(&self.0, applet_resource_user_id)
    }

    /// Cancels audio device connection rejection (cmd 23, 13.0.0+).
    ///
    /// Sends PID and the applet resource user ID.
    #[inline]
    pub fn cancel_audio_device_connection_rejection(
        &self,
        applet_resource_user_id: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::cancel_audio_device_connection_rejection(&self.0, applet_resource_user_id)
    }
}

// ---------------------------------------------------------------------------
// Connection
// ---------------------------------------------------------------------------

/// Connects to the Bluetooth Manager System service (`btm:sys`) using CMIF.
///
/// Obtains the root `btm:sys` session, then extracts the IBtmSystemCore
/// sub-object (cmd 0). The root session is closed automatically on `Drop`.
pub fn connect_cmif(sm: &SmService) -> Result<BtmsysService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    let root = Session::new(handle, 0);

    let core_raw = cmif::get_core(&root).map_err(ConnectCmifError::GetCore)?;

    // SAFETY: the kernel returned a valid move handle for the new IBtmSystemCore
    // sub-object; ownership transfers to the new `Session`.
    let core_handle = Handle::from_raw_unchecked(core_raw);

    Ok(BtmsysService(Session::new(
        OwnedSessionHandle::from_handle_unchecked(core_handle),
        0,
    )))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    #[error("failed to get btm:sys service")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    #[error("failed to get IBtmSystemCore sub-object")]
    GetCore(#[source] cmif::GetCoreError),
}
