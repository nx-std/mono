//! Audio output service (`audout:u`, `audout:a`, `audout:d`) implementation.
//!
//! Provides access to audio output devices for playing audio through
//! the built-in speakers, headphone jack, or USB audio.
//!
//! ## Architecture
//!
//! Three service interfaces are exposed:
//!
//! - **`audout:u`** (user): Root session for listing and opening audio output
//!   devices. [`connect_cmif`] obtains the root session, then
//!   [`AudoutService::open_audio_out`] (or its legacy variant) returns an
//!   [`AudoutAudioOut`] with its own independent session handle.
//! - **`audout:a`** (admin, pre-11.0.0): Suspend/resume audio and get/set
//!   per-process volume. Replaced by `aud:a` in 11.0.0+.
//! - **`audout:d`** (debug, pre-11.0.0): Debug suspend/resume for audio.
//!   Replaced by `aud:d` in 11.0.0+.
//!
//! ## Divergence from libnx
//!
//! libnx's `audout.c` keeps guarded global singletons managed by
//! `NX_GENERATE_SERVICE_GUARD`, auto-opens the default device on init,
//! and auto-registers the buffer event. This crate follows the convention
//! of the other `nx-service-*` crates: connect once via [`connect_cmif`],
//! open a device explicitly, and register the buffer event manually.
//!
//! Per IC-4, this crate is hosversion-unaware. Commands that differ across
//! firmware versions are exposed as paired methods:
//!
//! - [`AudoutService::list_audio_outs`] / [`AudoutService::list_audio_outs_legacy`]
//! - [`AudoutService::open_audio_out`] / [`AudoutService::open_audio_out_legacy`]
//! - [`AudoutAudioOut::append_buffer`] / [`AudoutAudioOut::append_buffer_legacy`]
//! - [`AudoutAudioOut::get_released_buffer`] / [`AudoutAudioOut::get_released_buffer_legacy`]

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{DispatchError, Session};
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::{
        AppendBufferError, ContainsBufferError, GetReleasedBufferError, GetVolumeError,
        ListAudioOutsError, OpenAudioOutError, SetVolumeError, SuspendResumeError,
    },
    proto::{AUDOUTA_SERVICE_NAME, AUDOUTD_SERVICE_NAME, DEVICE_NAME_LENGTH, SERVICE_NAME},
    types::{AudioOutBuffer, AudioOutState, OpenAudioOutOut},
};

/// Audio output (`audout:u`) root session wrapper.
///
/// Use [`list_audio_outs`](Self::list_audio_outs) to enumerate devices and
/// [`open_audio_out`](Self::open_audio_out) to create an audio output session.
#[repr(transparent)]
pub struct AudoutService(Session);

impl AudoutService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// Root service commands for `AudoutService`.
impl AudoutService {
    /// Lists available audio output devices (auto-select). \[3.0.0+\]
    ///
    /// `device_names_buf` should be `count * DEVICE_NAME_LENGTH` bytes.
    /// Returns the number of device names written.
    #[inline]
    pub fn list_audio_outs(&self, device_names_buf: &mut [u8]) -> Result<u32, ListAudioOutsError> {
        cmif::list_audio_outs(&self.0, device_names_buf)
    }

    /// Lists available audio output devices (map-alias, legacy). \[1.0.0-2.x.x\]
    ///
    /// `device_names_buf` should be `count * DEVICE_NAME_LENGTH` bytes.
    /// Returns the number of device names written.
    #[inline]
    pub fn list_audio_outs_legacy(
        &self,
        device_names_buf: &mut [u8],
    ) -> Result<u32, ListAudioOutsError> {
        cmif::list_audio_outs_legacy(&self.0, device_names_buf)
    }

    /// Opens an audio output device (auto-select). \[3.0.0+\]
    ///
    /// `device_name_in` is a 0x100-byte device name (empty for default).
    /// `device_name_out` receives the opened device name (0x100 bytes).
    /// `sample_rate` and `channel_count` are the desired audio parameters.
    ///
    /// Returns the audio-out session wrapper and negotiated output parameters.
    pub fn open_audio_out(
        &self,
        device_name_in: &[u8; DEVICE_NAME_LENGTH],
        device_name_out: &mut [u8; DEVICE_NAME_LENGTH],
        sample_rate: u32,
        channel_count: u32,
    ) -> Result<(AudoutAudioOut, OpenAudioOutOut), OpenAudioOutError> {
        let input = types::OpenAudioOutIn {
            sample_rate,
            channel_count,
            applet_resource_user_id: 0,
        };
        let (raw_handle, out) =
            cmif::open_audio_out(&self.0, &input, device_name_in, device_name_out)?;

        // SAFETY: the kernel returned a valid move handle for the
        // IAudioOut session; ownership transfers to the new `Session`.
        let session = unsafe { SessionHandle::from_raw(raw_handle) };
        let service = Session::from_handle(session, 0);

        Ok((AudoutAudioOut(service), out))
    }

    /// Opens an audio output device (map-alias, legacy). \[1.0.0-2.x.x\]
    ///
    /// `device_name_in` is a 0x100-byte device name (empty for default).
    /// `device_name_out` receives the opened device name (0x100 bytes).
    /// `sample_rate` and `channel_count` are the desired audio parameters.
    ///
    /// Returns the audio-out session wrapper and negotiated output parameters.
    pub fn open_audio_out_legacy(
        &self,
        device_name_in: &[u8; DEVICE_NAME_LENGTH],
        device_name_out: &mut [u8; DEVICE_NAME_LENGTH],
        sample_rate: u32,
        channel_count: u32,
    ) -> Result<(AudoutAudioOut, OpenAudioOutOut), OpenAudioOutError> {
        let input = types::OpenAudioOutIn {
            sample_rate,
            channel_count,
            applet_resource_user_id: 0,
        };
        let (raw_handle, out) =
            cmif::open_audio_out_legacy(&self.0, &input, device_name_in, device_name_out)?;

        // SAFETY: the kernel returned a valid move handle for the
        // IAudioOut session; ownership transfers to the new `Session`.
        let session = unsafe { SessionHandle::from_raw(raw_handle) };
        let service = Session::from_handle(session, 0);

        Ok((AudoutAudioOut(service), out))
    }
}

/// Audio output session wrapper (IAudioOut).
///
/// Obtained via [`AudoutService::open_audio_out`]. Owns its own independent
/// session handle.
#[repr(transparent)]
pub struct AudoutAudioOut(Session);

impl AudoutAudioOut {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods for `AudoutAudioOut`.
impl AudoutAudioOut {
    /// Gets the current audio output state as a raw `u32`.
    ///
    /// Returns `0` for [`AudioOutState::Started`] or `1` for
    /// [`AudioOutState::Stopped`].
    #[inline]
    pub fn get_state(&self) -> Result<u32, DispatchError> {
        cmif::audio_out_get_state(&self.0)
    }

    /// Starts audio output playback.
    #[inline]
    pub fn start(&self) -> Result<(), DispatchError> {
        cmif::audio_out_start(&self.0)
    }

    /// Stops audio output playback.
    #[inline]
    pub fn stop(&self) -> Result<(), DispatchError> {
        cmif::audio_out_stop(&self.0)
    }

    /// Registers the buffer event and returns a copy-handle for the event.
    ///
    /// The returned handle can be used with event-waiting primitives to know
    /// when a buffer has been played and released.
    #[inline]
    pub fn register_buffer_event(&self) -> Result<u32, DispatchError> {
        cmif::audio_out_register_buffer_event(&self.0)
    }

    /// Appends an audio output buffer (auto-select). \[3.0.0+\]
    ///
    /// `buffer_client_ptr` is the client-side pointer identifying this buffer.
    /// `buffer` describes the buffer layout.
    #[inline]
    pub fn append_buffer(
        &self,
        buffer_client_ptr: u64,
        buffer: &AudioOutBuffer,
    ) -> Result<(), AppendBufferError> {
        cmif::audio_out_append_buffer(&self.0, buffer_client_ptr, buffer)
    }

    /// Appends an audio output buffer (map-alias, legacy). \[1.0.0-2.x.x\]
    ///
    /// `buffer_client_ptr` is the client-side pointer identifying this buffer.
    /// `buffer` describes the buffer layout.
    #[inline]
    pub fn append_buffer_legacy(
        &self,
        buffer_client_ptr: u64,
        buffer: &AudioOutBuffer,
    ) -> Result<(), AppendBufferError> {
        cmif::audio_out_append_buffer_legacy(&self.0, buffer_client_ptr, buffer)
    }

    /// Gets a released audio output buffer (auto-select). \[3.0.0+\]
    ///
    /// Writes the released buffer's client-side pointer to `out_buffer_ptr`.
    /// Returns the number of released buffers.
    #[inline]
    pub fn get_released_buffer(
        &self,
        out_buffer_ptr: &mut u64,
    ) -> Result<u32, GetReleasedBufferError> {
        cmif::audio_out_get_released_buffer(&self.0, out_buffer_ptr)
    }

    /// Gets a released audio output buffer (map-alias, legacy). \[1.0.0-2.x.x\]
    ///
    /// Writes the released buffer's client-side pointer to `out_buffer_ptr`.
    /// Returns the number of released buffers.
    #[inline]
    pub fn get_released_buffer_legacy(
        &self,
        out_buffer_ptr: &mut u64,
    ) -> Result<u32, GetReleasedBufferError> {
        cmif::audio_out_get_released_buffer_legacy(&self.0, out_buffer_ptr)
    }

    /// Checks whether a buffer is contained in the audio output.
    ///
    /// `buffer_client_ptr` is the client-side pointer of the buffer to check.
    #[inline]
    pub fn contains_buffer(&self, buffer_client_ptr: u64) -> Result<bool, ContainsBufferError> {
        cmif::audio_out_contains_buffer(&self.0, buffer_client_ptr)
    }

    /// Gets the number of queued audio output buffers. \[4.0.0+\]
    #[inline]
    pub fn get_buffer_count(&self) -> Result<u32, DispatchError> {
        cmif::audio_out_get_buffer_count(&self.0)
    }

    /// Gets the total number of played samples. \[4.0.0+\]
    #[inline]
    pub fn get_played_sample_count(&self) -> Result<u64, DispatchError> {
        cmif::audio_out_get_played_sample_count(&self.0)
    }

    /// Flushes all queued audio output buffers. \[4.0.0+\]
    ///
    /// Returns whether any buffers were flushed.
    #[inline]
    pub fn flush_buffers(&self) -> Result<bool, DispatchError> {
        cmif::audio_out_flush_buffers(&self.0)
    }

    /// Sets the audio output volume. \[6.0.0+\]
    #[inline]
    pub fn set_volume(&self, volume: f32) -> Result<(), SetVolumeError> {
        cmif::audio_out_set_volume(&self.0, volume)
    }

    /// Gets the audio output volume. \[6.0.0+\]
    #[inline]
    pub fn get_volume(&self) -> Result<f32, GetVolumeError> {
        cmif::audio_out_get_volume(&self.0)
    }
}

// ---------------------------------------------------------------------------
// audout:a (Audio Output Admin, pre-11.0.0)
// ---------------------------------------------------------------------------

/// Audio output admin (`audout:a`) session wrapper.
///
/// Provides suspend/resume and volume control for audio output processes.
/// Removed in 11.0.0 (replaced by `aud:a`).
#[repr(transparent)]
pub struct AudoutaService(Session);

impl AudoutaService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods for `audout:a`.
impl AudoutaService {
    /// Suspends audio output for a process. \[4.0.0+\]
    #[inline]
    pub fn request_suspend(&self, pid: u64, delay: u64) -> Result<(), SuspendResumeError> {
        cmif::audouta_request_suspend(&self.0, pid, delay)
    }

    /// Resumes audio output for a process. \[4.0.0+\]
    #[inline]
    pub fn request_resume(&self, pid: u64, delay: u64) -> Result<(), SuspendResumeError> {
        cmif::audouta_request_resume(&self.0, pid, delay)
    }

    /// Gets the master volume for a process.
    #[inline]
    pub fn get_process_master_volume(&self, pid: u64) -> Result<f32, GetVolumeError> {
        cmif::audouta_get_process_master_volume(&self.0, pid)
    }

    /// Sets the master volume for a process.
    #[inline]
    pub fn set_process_master_volume(
        &self,
        pid: u64,
        delay: u64,
        volume: f32,
    ) -> Result<(), SetVolumeError> {
        cmif::audouta_set_process_master_volume(&self.0, pid, delay, volume)
    }

    /// Gets the record volume for a process. \[4.0.0+\]
    #[inline]
    pub fn get_process_record_volume(&self, pid: u64) -> Result<f32, GetVolumeError> {
        cmif::audouta_get_process_record_volume(&self.0, pid)
    }

    /// Sets the record volume for a process. \[4.0.0+\]
    #[inline]
    pub fn set_process_record_volume(
        &self,
        pid: u64,
        delay: u64,
        volume: f32,
    ) -> Result<(), SetVolumeError> {
        cmif::audouta_set_process_record_volume(&self.0, pid, delay, volume)
    }
}

// ---------------------------------------------------------------------------
// audout:d (Audio Output Debug, pre-11.0.0)
// ---------------------------------------------------------------------------

/// Audio output debug (`audout:d`) session wrapper.
///
/// Provides debug suspend/resume control for audio output processes.
/// Removed in 11.0.0 (replaced by `aud:d`).
#[repr(transparent)]
pub struct AudoutdService(Session);

impl AudoutdService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods for `audout:d`.
impl AudoutdService {
    /// Suspends audio output for a process (debug).
    #[inline]
    pub fn request_suspend_for_debug(
        &self,
        pid: u64,
        delay: u64,
    ) -> Result<(), SuspendResumeError> {
        cmif::audoutd_request_suspend_for_debug(&self.0, pid, delay)
    }

    /// Resumes audio output for a process (debug).
    #[inline]
    pub fn request_resume_for_debug(&self, pid: u64, delay: u64) -> Result<(), SuspendResumeError> {
        cmif::audoutd_request_resume_for_debug(&self.0, pid, delay)
    }
}

// ---------------------------------------------------------------------------
// Connect functions
// ---------------------------------------------------------------------------

/// Connects to the `audout:u` service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<AudoutService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(AudoutService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get audout:u service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

/// Connects to the `audout:a` (Audio Output Admin) service using CMIF.
///
/// Removed in 11.0.0 (replaced by `aud:a`).
pub fn connect_audouta_cmif(sm: &SmService) -> Result<AudoutaService, ConnectAudoutaCmifError> {
    let handle = sm
        .get_service_handle_cmif(AUDOUTA_SERVICE_NAME)
        .map_err(ConnectAudoutaCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(AudoutaService(service))
}

/// Error returned by [`connect_audouta_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get audout:a service")]
pub struct ConnectAudoutaCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

/// Connects to the `audout:d` (Audio Output Debug) service using CMIF.
///
/// Removed in 11.0.0 (replaced by `aud:d`).
pub fn connect_audoutd_cmif(sm: &SmService) -> Result<AudoutdService, ConnectAudoutdCmifError> {
    let handle = sm
        .get_service_handle_cmif(AUDOUTD_SERVICE_NAME)
        .map_err(ConnectAudoutdCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(AudoutdService(service))
}

/// Error returned by [`connect_audoutd_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get audout:d service")]
pub struct ConnectAudoutdCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
