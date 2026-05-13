//! Audio input service (`audin:u`) implementation.
//!
//! Provides access to audio input devices for capturing audio from
//! the built-in microphone or headset.
//!
//! ## Architecture
//!
//! The service is non-domain. [`connect_cmif`] obtains the root session,
//! then [`AudinService::open_audio_in`] (or its legacy variant) returns an
//! [`AudinAudioIn`] with its own independent session handle.
//!
//! ## Divergence from libnx
//!
//! libnx's `audin.c` keeps a guarded global singleton managed by
//! `NX_GENERATE_SERVICE_GUARD`, auto-opens the default device on init,
//! and auto-registers the buffer event. This crate follows the convention
//! of the other `nx-service-*` crates: connect once via [`connect_cmif`],
//! open a device explicitly, and register the buffer event manually.
//!
//! Per IC-4, this crate is hosversion-unaware. Commands that differ across
//! firmware versions are exposed as paired methods:
//!
//! - [`AudinService::list_audio_ins`] / [`AudinService::list_audio_ins_legacy`]
//! - [`AudinService::open_audio_in`] / [`AudinService::open_audio_in_legacy`]
//! - [`AudinAudioIn::append_buffer`] / [`AudinAudioIn::append_buffer_legacy`]
//! - [`AudinAudioIn::get_released_buffer`] / [`AudinAudioIn::get_released_buffer_legacy`]

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
        AppendBufferError, ContainsBufferError, GetReleasedBufferError, ListAudioInsError,
        OpenAudioInError,
    },
    proto::{DEVICE_NAME_LENGTH, SERVICE_NAME},
    types::{AudioInBuffer, AudioInState, OpenAudioInOut},
};

/// Audio input (`audin:u`) root session wrapper.
///
/// Use [`list_audio_ins`](Self::list_audio_ins) to enumerate devices and
/// [`open_audio_in`](Self::open_audio_in) to create an audio input session.
#[repr(transparent)]
pub struct AudinService(Session);

impl AudinService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// Root service commands for `AudinService`.
impl AudinService {
    /// Lists available audio input devices (auto-select). \[3.0.0+\]
    ///
    /// `device_names_buf` should be `count * DEVICE_NAME_LENGTH` bytes.
    /// Returns the number of device names written.
    #[inline]
    pub fn list_audio_ins(&self, device_names_buf: &mut [u8]) -> Result<u32, ListAudioInsError> {
        cmif::list_audio_ins(&self.0, device_names_buf)
    }

    /// Lists available audio input devices (map-alias, legacy). \[1.0.0-2.x.x\]
    ///
    /// `device_names_buf` should be `count * DEVICE_NAME_LENGTH` bytes.
    /// Returns the number of device names written.
    #[inline]
    pub fn list_audio_ins_legacy(
        &self,
        device_names_buf: &mut [u8],
    ) -> Result<u32, ListAudioInsError> {
        cmif::list_audio_ins_legacy(&self.0, device_names_buf)
    }

    /// Opens an audio input device (auto-select). \[3.0.0+\]
    ///
    /// `device_name_in` is a 0x100-byte device name (empty for default).
    /// `device_name_out` receives the opened device name (0x100 bytes).
    /// `sample_rate` and `channel_count` are the desired audio parameters.
    ///
    /// Returns the audio-in session wrapper and negotiated output parameters.
    pub fn open_audio_in(
        &self,
        device_name_in: &[u8; DEVICE_NAME_LENGTH],
        device_name_out: &mut [u8; DEVICE_NAME_LENGTH],
        sample_rate: u32,
        channel_count: u32,
    ) -> Result<(AudinAudioIn, OpenAudioInOut), OpenAudioInError> {
        let input = types::OpenAudioInIn {
            sample_rate,
            channel_count,
            client_pid: 0,
        };
        let (raw_handle, out) =
            cmif::open_audio_in(&self.0, &input, device_name_in, device_name_out)?;

        // SAFETY: the kernel returned a valid move handle for the
        // IAudioIn session; ownership transfers to the new `Session`.
        let session = unsafe { SessionHandle::from_raw(raw_handle) };
        let service = Session::from_handle(session, 0);

        Ok((AudinAudioIn(service), out))
    }

    /// Opens an audio input device (map-alias, legacy). \[1.0.0-2.x.x\]
    ///
    /// `device_name_in` is a 0x100-byte device name (empty for default).
    /// `device_name_out` receives the opened device name (0x100 bytes).
    /// `sample_rate` and `channel_count` are the desired audio parameters.
    ///
    /// Returns the audio-in session wrapper and negotiated output parameters.
    pub fn open_audio_in_legacy(
        &self,
        device_name_in: &[u8; DEVICE_NAME_LENGTH],
        device_name_out: &mut [u8; DEVICE_NAME_LENGTH],
        sample_rate: u32,
        channel_count: u32,
    ) -> Result<(AudinAudioIn, OpenAudioInOut), OpenAudioInError> {
        let input = types::OpenAudioInIn {
            sample_rate,
            channel_count,
            client_pid: 0,
        };
        let (raw_handle, out) =
            cmif::open_audio_in_legacy(&self.0, &input, device_name_in, device_name_out)?;

        // SAFETY: the kernel returned a valid move handle for the
        // IAudioIn session; ownership transfers to the new `Session`.
        let session = unsafe { SessionHandle::from_raw(raw_handle) };
        let service = Session::from_handle(session, 0);

        Ok((AudinAudioIn(service), out))
    }
}

/// Audio input session wrapper (IAudioIn).
///
/// Obtained via [`AudinService::open_audio_in`]. Owns its own independent
/// session handle.
#[repr(transparent)]
pub struct AudinAudioIn(Session);

impl AudinAudioIn {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods for `AudinAudioIn`.
impl AudinAudioIn {
    /// Gets the current audio input state as a raw `u32`.
    ///
    /// Returns `0` for [`AudioInState::Started`] or `1` for
    /// [`AudioInState::Stopped`].
    #[inline]
    pub fn get_state(&self) -> Result<u32, DispatchError> {
        cmif::audio_in_get_state(&self.0)
    }

    /// Starts audio input capture.
    #[inline]
    pub fn start(&self) -> Result<(), DispatchError> {
        cmif::audio_in_start(&self.0)
    }

    /// Stops audio input capture.
    #[inline]
    pub fn stop(&self) -> Result<(), DispatchError> {
        cmif::audio_in_stop(&self.0)
    }

    /// Registers the buffer event and returns a copy-handle for the event.
    ///
    /// The returned handle can be used with event-waiting primitives to know
    /// when a buffer has been captured and released.
    #[inline]
    pub fn register_buffer_event(&self) -> Result<u32, DispatchError> {
        cmif::audio_in_register_buffer_event(&self.0)
    }

    /// Appends an audio input buffer (auto-select). \[3.0.0+\]
    ///
    /// `buffer_client_ptr` is the client-side pointer identifying this buffer.
    /// `buffer` describes the buffer layout.
    #[inline]
    pub fn append_buffer(
        &self,
        buffer_client_ptr: u64,
        buffer: &AudioInBuffer,
    ) -> Result<(), AppendBufferError> {
        cmif::audio_in_append_buffer(&self.0, buffer_client_ptr, buffer)
    }

    /// Appends an audio input buffer (map-alias, legacy). \[1.0.0-2.x.x\]
    ///
    /// `buffer_client_ptr` is the client-side pointer identifying this buffer.
    /// `buffer` describes the buffer layout.
    #[inline]
    pub fn append_buffer_legacy(
        &self,
        buffer_client_ptr: u64,
        buffer: &AudioInBuffer,
    ) -> Result<(), AppendBufferError> {
        cmif::audio_in_append_buffer_legacy(&self.0, buffer_client_ptr, buffer)
    }

    /// Gets a released audio input buffer (auto-select). \[3.0.0+\]
    ///
    /// Writes the released buffer's client-side pointer to `out_buffer_ptr`.
    /// Returns the number of released buffers.
    #[inline]
    pub fn get_released_buffer(
        &self,
        out_buffer_ptr: &mut u64,
    ) -> Result<u32, GetReleasedBufferError> {
        cmif::audio_in_get_released_buffer(&self.0, out_buffer_ptr)
    }

    /// Gets a released audio input buffer (map-alias, legacy). \[1.0.0-2.x.x\]
    ///
    /// Writes the released buffer's client-side pointer to `out_buffer_ptr`.
    /// Returns the number of released buffers.
    #[inline]
    pub fn get_released_buffer_legacy(
        &self,
        out_buffer_ptr: &mut u64,
    ) -> Result<u32, GetReleasedBufferError> {
        cmif::audio_in_get_released_buffer_legacy(&self.0, out_buffer_ptr)
    }

    /// Checks whether a buffer is contained in the audio input.
    ///
    /// `buffer_client_ptr` is the client-side pointer of the buffer to check.
    #[inline]
    pub fn contains_buffer(&self, buffer_client_ptr: u64) -> Result<bool, ContainsBufferError> {
        cmif::audio_in_contains_buffer(&self.0, buffer_client_ptr)
    }
}

/// Connects to the `audin:u` service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<AudinService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(AudinService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get audin:u service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
