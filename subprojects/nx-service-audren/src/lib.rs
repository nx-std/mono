//! Audio renderer service (`audren:u`) implementation.
//!
//! Provides access to the audio renderer for mixing and playing audio on
//! the Nintendo Switch.
//!
//! ## Architecture
//!
//! - **`audren:u`** (user): Root session (IAudioRendererManager) for creating
//!   audio renderers and querying work buffer sizes. [`connect_cmif`] obtains
//!   the root session, then [`AudrenService::open_audio_renderer`] returns an
//!   [`AudrenRenderer`] with its own independent session handle.
//!
//! ## Divergence from libnx
//!
//! libnx's `audren.c` uses a guarded global singleton managed by
//! `NX_GENERATE_SERVICE_GUARD`, auto-selects the revision based on
//! `hosversion`, and allocates transfer memory internally. This crate
//! follows the convention of the other `nx-service-*` crates: connect
//! once via [`connect_cmif`], pass transfer memory and revision
//! externally, and manage the renderer lifecycle explicitly.
//!
//! Per IC-4, this crate is hosversion-unaware. The caller selects the
//! appropriate revision constant and paired method variant:
//!
//! - [`AudrenRenderer::request_update`] / [`AudrenRenderer::request_update_legacy`]

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
    cmif::{GetWorkBufferSizeError, OpenAudioRendererError, RequestUpdateError},
    proto::SERVICE_NAME,
    types::{
        AdpcmContext, AdpcmParameters, AudioRendererParameter, BUFFER_ALIGNMENT, BehaviorInfoIn,
        BehaviorInfoOut, BiquadFilter, ChannelInfoIn, CircularBufferSinkInfoIn, DeviceSinkInfoIn,
        DownMixParameters, FINAL_MIX_ID, INPUT_PARAM_ALIGNMENT, MEMPOOL_ALIGNMENT, MemPoolInfoIn,
        MemPoolInfoOut, MemPoolState, MixInfoIn, OUTPUT_PARAM_ALIGNMENT, OutputRate, PcmFormat,
        PerformanceBufferInfoIn, PerformanceBufferInfoOut, REVISION_1, REVISION_2, REVISION_3,
        REVISION_4, REVISION_5, REVISION_6, SAMPLES_PER_FRAME_32KHZ, SAMPLES_PER_FRAME_48KHZ,
        SinkInfoIn, SinkInfoOut, SinkType, TIMER_FREQ_HZ, TIMER_PERIOD_MS, UNUSED_MIX_ID,
        UNUSED_SPLITTER_ID, UpdateDataHeader, VoiceInfoIn, VoiceInfoOut, VoicePlayState, WaveBuf,
    },
};

/// Audio renderer manager (`audren:u`) root session wrapper.
///
/// Use [`get_work_buffer_size`](Self::get_work_buffer_size) to query the
/// required buffer size, then [`open_audio_renderer`](Self::open_audio_renderer)
/// to create a renderer.
#[repr(transparent)]
pub struct AudrenService(Session);

impl AudrenService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// IAudioRendererManager commands.
impl AudrenService {
    /// Opens an audio renderer. \[1.0.0+\]
    ///
    /// `param` describes the renderer configuration (sample rate, voice
    /// count, etc.). `work_buffer_size` is obtained from
    /// [`get_work_buffer_size`](Self::get_work_buffer_size).
    /// `aruid` is the applet resource user ID. `tmem_handle` is the
    /// transfer-memory handle for the work buffer. `process_handle` is
    /// the current process handle.
    ///
    /// Returns an [`AudrenRenderer`] wrapping the IAudioRenderer session.
    pub fn open_audio_renderer(
        &self,
        param: &AudioRendererParameter,
        work_buffer_size: u64,
        aruid: u64,
        tmem_handle: u32,
        process_handle: u32,
    ) -> Result<AudrenRenderer, OpenAudioRendererError> {
        let raw_handle = cmif::open_audio_renderer(
            &self.0,
            param,
            work_buffer_size,
            aruid,
            tmem_handle,
            process_handle,
        )?;

        // SAFETY: the kernel returned a valid move handle for the
        // IAudioRenderer session; ownership transfers to the new `Session`.
        let session = unsafe { SessionHandle::from_raw(raw_handle) };
        let service = Session::from_handle(session, 0);

        Ok(AudrenRenderer(service))
    }

    /// Gets the required work buffer size for the given parameters. \[1.0.0+\]
    #[inline]
    pub fn get_work_buffer_size(
        &self,
        param: &AudioRendererParameter,
    ) -> Result<u64, GetWorkBufferSizeError> {
        cmif::get_work_buffer_size(&self.0, param)
    }
}

// ---------------------------------------------------------------------------
// IAudioRenderer sub-object
// ---------------------------------------------------------------------------

/// Audio renderer session wrapper (IAudioRenderer).
///
/// Obtained via [`AudrenService::open_audio_renderer`]. Owns its own
/// independent session handle.
#[repr(transparent)]
pub struct AudrenRenderer(Session);

impl AudrenRenderer {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods for `AudrenRenderer`.
impl AudrenRenderer {
    /// Gets the current renderer state as a raw `u32`. \[1.0.0+\]
    #[inline]
    pub fn get_state(&self) -> Result<u32, DispatchError> {
        cmif::renderer_get_state(&self.0)
    }

    /// Requests update of the audio renderer (auto-select). \[3.0.0+\]
    ///
    /// `in_param_buf` is the serialized input parameter buffer.
    /// `out_param_buf` receives the output parameters.
    /// `perf_buf` receives performance data.
    #[inline]
    pub fn request_update(
        &self,
        in_param_buf: &[u8],
        out_param_buf: &mut [u8],
        perf_buf: &mut [u8],
    ) -> Result<(), RequestUpdateError> {
        cmif::renderer_request_update(&self.0, in_param_buf, out_param_buf, perf_buf)
    }

    /// Requests update of the audio renderer (map-alias, legacy).
    /// \[1.0.0-2.x.x\]
    ///
    /// `in_param_buf` is the serialized input parameter buffer.
    /// `out_param_buf` receives the output parameters.
    /// `perf_buf` receives performance data.
    #[inline]
    pub fn request_update_legacy(
        &self,
        in_param_buf: &[u8],
        out_param_buf: &mut [u8],
        perf_buf: &mut [u8],
    ) -> Result<(), RequestUpdateError> {
        cmif::renderer_request_update_legacy(&self.0, in_param_buf, out_param_buf, perf_buf)
    }

    /// Starts the audio renderer. \[1.0.0+\]
    #[inline]
    pub fn start(&self) -> Result<(), DispatchError> {
        cmif::renderer_start(&self.0)
    }

    /// Stops the audio renderer. \[1.0.0+\]
    #[inline]
    pub fn stop(&self) -> Result<(), DispatchError> {
        cmif::renderer_stop(&self.0)
    }

    /// Queries the system event (copy handle, autoclear). \[1.0.0+\]
    ///
    /// Returns the raw handle for the event, signalled on each new frame.
    #[inline]
    pub fn query_system_event(&self) -> Result<u32, DispatchError> {
        cmif::renderer_query_system_event(&self.0)
    }

    /// Sets the rendering time limit as a percentage. \[1.0.0+\]
    #[inline]
    pub fn set_rendering_time_limit(&self, percent: i32) -> Result<(), DispatchError> {
        cmif::renderer_set_rendering_time_limit(&self.0, percent)
    }
}

// ---------------------------------------------------------------------------
// Connect function
// ---------------------------------------------------------------------------

/// Connects to the `audren:u` service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<AudrenService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(AudrenService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get audren:u service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
