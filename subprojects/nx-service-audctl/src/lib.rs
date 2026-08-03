//! Audio control (`audctl`) service implementation.
//!
//! Provides audio volume, mute, output target, output mode, headphone level,
//! force-mute policy, system master volume, and play-report event acquisition
//! via the `audctl` IPC service.
//!
//! ## Hosversion variants
//!
//! Several commands have version restrictions in libnx:
//!
//! - `is_target_connected`: pre-18.0.0 only
//! - `set_force_mute_policy` / `get_force_mute_policy`: pre-14.0.0 only
//! - `set_headphone_output_level_mode` / `get_headphone_output_level_mode`: 3.0.0+
//! - `acquire_audio_volume_update_event_for_play_report`: 3.0.0–13.2.1
//! - `acquire_audio_output_device_update_event_for_play_report`: 3.0.0–13.2.1
//! - `get_audio_output_target_for_play_report`: 3.0.0+
//! - `notify_headphone_volume_warning_displayed_event`: 3.0.0+
//! - `set_system_output_master_volume` / `get_system_output_master_volume`: 4.0.0+
//! - `get_active_output_target`: 13.0.0+
//!
//! This crate exposes all commands unconditionally and leaves version
//! selection to the caller.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{BorrowedSessionHandle, Session};

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{
        DispatchEventError, DispatchInBoolError, DispatchInF32Error, DispatchInStructError,
        DispatchInU32Error, DispatchInU32OutBoolError, DispatchInU32OutI32Error,
        DispatchInU32OutU32Error, DispatchNoIoError, DispatchOutF32Error, DispatchOutI32Error,
        DispatchOutU32Error,
    },
    proto::SERVICE_NAME,
    types::{AudioForceMutePolicy, AudioHeadphoneOutputLevelMode, AudioOutputMode, AudioTarget},
};

/// Audio control service (`audctl`) session wrapper.
#[repr(transparent)]
pub struct AudctlService(Session);

impl AudctlService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// Volume control.
impl AudctlService {
    /// Gets the volume for the specified target.
    #[inline]
    pub fn get_target_volume(&self, target: AudioTarget) -> Result<i32, DispatchInU32OutI32Error> {
        cmif::get_target_volume(self.0.handle(), target as u32)
    }

    /// Sets the volume for the specified target.
    #[inline]
    pub fn set_target_volume(
        &self,
        target: AudioTarget,
        volume: i32,
    ) -> Result<(), DispatchInStructError> {
        cmif::set_target_volume(self.0.handle(), target as u32, volume)
    }

    /// Gets the minimum volume value.
    #[inline]
    pub fn get_target_volume_min(&self) -> Result<i32, DispatchOutI32Error> {
        cmif::get_target_volume_min(self.0.handle())
    }

    /// Gets the maximum volume value.
    #[inline]
    pub fn get_target_volume_max(&self) -> Result<i32, DispatchOutI32Error> {
        cmif::get_target_volume_max(self.0.handle())
    }
}

/// Mute control.
impl AudctlService {
    /// Returns whether the specified target is muted.
    #[inline]
    pub fn is_target_mute(&self, target: AudioTarget) -> Result<bool, DispatchInU32OutBoolError> {
        cmif::is_target_mute(self.0.handle(), target as u32)
    }

    /// Sets the mute state for the specified target.
    #[inline]
    pub fn set_target_mute(
        &self,
        target: AudioTarget,
        mute: bool,
    ) -> Result<(), DispatchInStructError> {
        cmif::set_target_mute(self.0.handle(), target as u32, mute)
    }
}

/// Target connection and selection.
impl AudctlService {
    /// Returns whether the specified target is connected (pre-18.0.0).
    #[inline]
    pub fn is_target_connected(
        &self,
        target: AudioTarget,
    ) -> Result<bool, DispatchInU32OutBoolError> {
        cmif::is_target_connected(self.0.handle(), target as u32)
    }

    /// Sets the default audio target with fade durations (nanoseconds).
    #[inline]
    pub fn set_default_target(
        &self,
        target: AudioTarget,
        fade_in_ns: u64,
        fade_out_ns: u64,
    ) -> Result<(), DispatchInStructError> {
        cmif::set_default_target(self.0.handle(), target as u32, fade_in_ns, fade_out_ns)
    }

    /// Gets the default audio target.
    ///
    /// Returns `None` if the service returns an unrecognised target value.
    #[inline]
    pub fn get_default_target(&self) -> Result<Option<AudioTarget>, DispatchOutU32Error> {
        let raw = cmif::get_default_target(self.0.handle())?;
        Ok(AudioTarget::from_raw(raw))
    }

    /// Sets the output target.
    #[inline]
    pub fn set_output_target(&self, target: AudioTarget) -> Result<(), DispatchInU32Error> {
        cmif::set_output_target(self.0.handle(), target as u32)
    }

    /// Gets the active output target (13.0.0+).
    ///
    /// Returns `None` if the service returns an unrecognised target value.
    #[inline]
    pub fn get_active_output_target(&self) -> Result<Option<AudioTarget>, DispatchOutU32Error> {
        let raw = cmif::get_active_output_target(self.0.handle())?;
        Ok(AudioTarget::from_raw(raw))
    }
}

/// Audio output mode.
impl AudctlService {
    /// Gets the audio output mode for the specified target.
    ///
    /// Returns `None` if the service returns an unrecognised mode value.
    #[inline]
    pub fn get_audio_output_mode(
        &self,
        target: AudioTarget,
    ) -> Result<Option<AudioOutputMode>, DispatchInU32OutU32Error> {
        let raw = cmif::get_audio_output_mode(self.0.handle(), target as u32)?;
        Ok(AudioOutputMode::from_raw(raw))
    }

    /// Sets the audio output mode for the specified target.
    #[inline]
    pub fn set_audio_output_mode(
        &self,
        target: AudioTarget,
        mode: AudioOutputMode,
    ) -> Result<(), DispatchInStructError> {
        cmif::set_audio_output_mode(self.0.handle(), target as u32, mode as u32)
    }

    /// Gets the output mode setting for the specified target.
    ///
    /// Returns `None` if the service returns an unrecognised mode value.
    #[inline]
    pub fn get_output_mode_setting(
        &self,
        target: AudioTarget,
    ) -> Result<Option<AudioOutputMode>, DispatchInU32OutU32Error> {
        let raw = cmif::get_output_mode_setting(self.0.handle(), target as u32)?;
        Ok(AudioOutputMode::from_raw(raw))
    }

    /// Sets the output mode setting for the specified target.
    #[inline]
    pub fn set_output_mode_setting(
        &self,
        target: AudioTarget,
        mode: AudioOutputMode,
    ) -> Result<(), DispatchInStructError> {
        cmif::set_output_mode_setting(self.0.handle(), target as u32, mode as u32)
    }
}

/// Force mute policy (pre-14.0.0).
impl AudctlService {
    /// Sets the force mute policy (pre-14.0.0).
    #[inline]
    pub fn set_force_mute_policy(
        &self,
        policy: AudioForceMutePolicy,
    ) -> Result<(), DispatchInU32Error> {
        cmif::set_force_mute_policy(self.0.handle(), policy as u32)
    }

    /// Gets the force mute policy (pre-14.0.0).
    ///
    /// Returns `None` if the service returns an unrecognised policy value.
    #[inline]
    pub fn get_force_mute_policy(
        &self,
    ) -> Result<Option<AudioForceMutePolicy>, DispatchOutU32Error> {
        let raw = cmif::get_force_mute_policy(self.0.handle())?;
        Ok(AudioForceMutePolicy::from_raw(raw))
    }
}

/// Input target and headphone control.
impl AudctlService {
    /// Sets whether the input target is force-enabled.
    #[inline]
    pub fn set_input_target_force_enabled(&self, enable: bool) -> Result<(), DispatchInBoolError> {
        cmif::set_input_target_force_enabled(self.0.handle(), enable)
    }

    /// Sets the headphone output level mode (3.0.0+).
    #[inline]
    pub fn set_headphone_output_level_mode(
        &self,
        mode: AudioHeadphoneOutputLevelMode,
    ) -> Result<(), DispatchInU32Error> {
        cmif::set_headphone_output_level_mode(self.0.handle(), mode as u32)
    }

    /// Gets the headphone output level mode (3.0.0+).
    ///
    /// Returns `None` if the service returns an unrecognised mode value.
    #[inline]
    pub fn get_headphone_output_level_mode(
        &self,
    ) -> Result<Option<AudioHeadphoneOutputLevelMode>, DispatchOutU32Error> {
        let raw = cmif::get_headphone_output_level_mode(self.0.handle())?;
        Ok(AudioHeadphoneOutputLevelMode::from_raw(raw))
    }
}

/// Play report events and queries.
impl AudctlService {
    /// Acquires the audio volume update event for play reports (3.0.0–13.2.1).
    ///
    /// Returns the raw copy handle for the event.
    #[inline]
    pub fn acquire_audio_volume_update_event_for_play_report(
        &self,
    ) -> Result<u32, DispatchEventError> {
        cmif::acquire_audio_volume_update_event_for_play_report(self.0.handle())
    }

    /// Acquires the audio output device update event for play reports
    /// (3.0.0–13.2.1).
    ///
    /// Returns the raw copy handle for the event.
    #[inline]
    pub fn acquire_audio_output_device_update_event_for_play_report(
        &self,
    ) -> Result<u32, DispatchEventError> {
        cmif::acquire_audio_output_device_update_event_for_play_report(self.0.handle())
    }

    /// Gets the audio output target for play reports (3.0.0+).
    ///
    /// Returns `None` if the service returns an unrecognised target value.
    #[inline]
    pub fn get_audio_output_target_for_play_report(
        &self,
    ) -> Result<Option<AudioTarget>, DispatchOutU32Error> {
        let raw = cmif::get_audio_output_target_for_play_report(self.0.handle())?;
        Ok(AudioTarget::from_raw(raw))
    }

    /// Notifies that the headphone volume warning has been displayed (3.0.0+).
    #[inline]
    pub fn notify_headphone_volume_warning_displayed_event(&self) -> Result<(), DispatchNoIoError> {
        cmif::notify_headphone_volume_warning_displayed_event(self.0.handle())
    }
}

/// System output master volume (4.0.0+).
impl AudctlService {
    /// Sets the system output master volume (4.0.0+).
    #[inline]
    pub fn set_system_output_master_volume(&self, volume: f32) -> Result<(), DispatchInF32Error> {
        cmif::set_system_output_master_volume(self.0.handle(), volume)
    }

    /// Gets the system output master volume (4.0.0+).
    #[inline]
    pub fn get_system_output_master_volume(&self) -> Result<f32, DispatchOutF32Error> {
        cmif::get_system_output_master_volume(self.0.handle())
    }
}

/// Connects to the `audctl` (Audio Control) service using CMIF.
///
/// The caller must close the returned [`AudctlService`] when done.
pub fn connect_cmif(sm: &SmService) -> Result<AudctlService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(AudctlService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get audctl service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
