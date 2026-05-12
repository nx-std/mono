//! IAudioDevice (`auddev`) service implementation.
//!
//! Provides audio device management — listing devices, querying and
//! setting output volume, and retrieving the active device name —
//! via the `IAudioDevice` interface obtained from `audren:u`.
//!
//! ## Connection
//!
//! Unlike most service crates, `auddev` is not a standalone SM
//! service. [`connect_cmif`] first connects to `audren:u`
//! (IAudioRendererManager), opens an `IAudioDevice` sub-object via
//! command 2 with the caller-provided applet resource user ID
//! (aruid), then closes the manager session.
//!
//! ## Hosversion variants
//!
//! The command surface changed at HOS 3.0.0. Pre-3.0.0 commands use
//! mapped buffers (Type A/B); 3.0.0+ commands use auto-select
//! buffers. This crate exposes both sets of methods (e.g.
//! [`list_audio_device_name`](AuddevService::list_audio_device_name) vs
//! [`list_audio_device_name_legacy`](AuddevService::list_audio_device_name_legacy))
//! and leaves version selection to the caller.
//!
//! ## Divergence from libnx
//!
//! libnx's `auddev.c` keeps a guarded global singleton
//! (`g_auddevIAudioDevice`) managed by `NX_GENERATE_SERVICE_GUARD`
//! and dispatches to legacy or modern commands at runtime via
//! `hosversionAtLeast(3,0,0)`. This crate follows the convention of
//! the other `nx-service-*` crates: connect once via [`connect_cmif`],
//! reuse the [`AuddevService`] across calls, and let the session
//! close on `Drop`. Hosversion gating is the caller's responsibility.

#![no_std]

extern crate nx_panic_handler;

use nx_service_sm::SmService;
use nx_sf::service::Session;
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{
        GetActiveAudioDeviceNameError, GetAudioDeviceOutputVolumeError, ListAudioDeviceNameError,
        SetAudioDeviceOutputVolumeError,
    },
    proto::SERVICE_NAME,
    types::AudioDeviceName,
};

/// IAudioDevice session wrapper.
#[repr(transparent)]
pub struct AuddevService(Session);

impl AuddevService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods (3.0.0+).
impl AuddevService {
    /// Lists audio device names (3.0.0+).
    ///
    /// Returns the number of names written to `names`.
    #[inline]
    pub fn list_audio_device_name(
        &self,
        names: &mut [AudioDeviceName],
    ) -> Result<i32, ListAudioDeviceNameError> {
        cmif::list_audio_device_name(self.0.handle(), names)
    }

    /// Sets the output volume for a named audio device (3.0.0+).
    #[inline]
    pub fn set_audio_device_output_volume(
        &self,
        device_name: &AudioDeviceName,
        volume: f32,
    ) -> Result<(), SetAudioDeviceOutputVolumeError> {
        cmif::set_audio_device_output_volume(self.0.handle(), device_name, volume)
    }

    /// Gets the output volume for a named audio device (3.0.0+).
    #[inline]
    pub fn get_audio_device_output_volume(
        &self,
        device_name: &AudioDeviceName,
    ) -> Result<f32, GetAudioDeviceOutputVolumeError> {
        cmif::get_audio_device_output_volume(self.0.handle(), device_name)
    }

    /// Gets the active audio device name (3.0.0+).
    #[inline]
    pub fn get_active_audio_device_name(
        &self,
        device_name: &mut AudioDeviceName,
    ) -> Result<(), GetActiveAudioDeviceNameError> {
        cmif::get_active_audio_device_name(self.0.handle(), device_name)
    }
}

/// CMIF protocol methods (legacy, pre-3.0.0).
impl AuddevService {
    /// Lists audio device names (legacy, pre-3.0.0).
    ///
    /// Returns the number of names written to `names`.
    #[inline]
    pub fn list_audio_device_name_legacy(
        &self,
        names: &mut [AudioDeviceName],
    ) -> Result<i32, ListAudioDeviceNameError> {
        cmif::list_audio_device_name_legacy(self.0.handle(), names)
    }

    /// Sets the output volume for a named audio device (legacy, pre-3.0.0).
    #[inline]
    pub fn set_audio_device_output_volume_legacy(
        &self,
        device_name: &AudioDeviceName,
        volume: f32,
    ) -> Result<(), SetAudioDeviceOutputVolumeError> {
        cmif::set_audio_device_output_volume_legacy(self.0.handle(), device_name, volume)
    }

    /// Gets the output volume for a named audio device (legacy, pre-3.0.0).
    #[inline]
    pub fn get_audio_device_output_volume_legacy(
        &self,
        device_name: &AudioDeviceName,
    ) -> Result<f32, GetAudioDeviceOutputVolumeError> {
        cmif::get_audio_device_output_volume_legacy(self.0.handle(), device_name)
    }

    /// Gets the active audio device name (legacy, pre-3.0.0).
    #[inline]
    pub fn get_active_audio_device_name_legacy(
        &self,
        device_name: &mut AudioDeviceName,
    ) -> Result<(), GetActiveAudioDeviceNameError> {
        cmif::get_active_audio_device_name_legacy(self.0.handle(), device_name)
    }
}

/// Connects to the IAudioDevice service using CMIF.
///
/// Opens the `audren:u` (IAudioRendererManager) service, requests an
/// `IAudioDevice` sub-object for the given applet resource user ID,
/// then closes the manager session.
///
/// The returned [`AuddevService`] closes its session on `Drop`.
pub fn connect_cmif(sm: &SmService, aruid: u64) -> Result<AuddevService, ConnectCmifError> {
    let mgr_handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    // Wrap the manager handle so it auto-closes on `Drop` regardless of the
    // success/error path — only the IAudioDevice sub-session is kept.
    let mgr = Session::from_handle(mgr_handle, 0);

    let device_handle = cmif::get_audio_device_service(mgr.handle(), aruid)
        .map_err(ConnectCmifError::OpenDevice)?;

    // Drop the manager session — only the IAudioDevice handle is needed.
    drop(mgr);

    Ok(AuddevService(Session::from_handle(device_handle, 0)))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    #[error("failed to get audren:u service")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    #[error("failed to open IAudioDevice")]
    OpenDevice(#[source] cmif::GetAudioDeviceServiceError),
}
