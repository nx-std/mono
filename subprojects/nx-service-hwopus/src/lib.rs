//! Hardware Opus decoder service (`hwopus`) implementation.
//!
//! Provides access to the hardware-accelerated Opus audio decoder on the
//! Nintendo Switch.
//!
//! ## Architecture
//!
//! The service is non-domain. [`connect_cmif`] obtains the manager session
//! (`IHardwareOpusDecoderManager`). The manager is used to query work buffer
//! sizes and open decoder sub-objects:
//!
//! - [`HwopusService::open_decoder`] — single-stream decoder
//! - [`HwopusService::open_decoder_multistream`] — multi-stream decoder
//!   \[3.0.0+\]
//!
//! Each [`HwopusDecoder`] owns an independent session handle and provides
//! three decode method variants per IC-4:
//!
//! - [`decode_interleaved_legacy`](HwopusDecoder::decode_interleaved_legacy)
//!   (pre-4.0.0)
//! - [`decode_interleaved_with_perf`](HwopusDecoder::decode_interleaved_with_perf)
//!   (4.0.0+)
//! - [`decode_interleaved`](HwopusDecoder::decode_interleaved) (6.0.0+)
//!
//! ## Divergence from libnx
//!
//! libnx's `hwopus.c` performs the full init sequence internally
//! (query work buffer size → create tmem → open decoder → close manager).
//! This crate separates manager and decoder lifecycle: the caller obtains the
//! manager, queries the work buffer size, creates transfer memory externally,
//! and opens the decoder. This gives callers full control over memory
//! management.
//!
//! Per IC-4, this crate is hosversion-unaware — callers choose the
//! appropriate decode method based on the target firmware version.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{DispatchError, Session};
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::OpenDecoderError,
    proto::SERVICE_NAME,
    types::{DecodeResult, DecodeResultWithPerf, HwopusHeader, HwopusMultistreamState},
};

/// Hardware Opus decoder manager (`hwopus`) session wrapper.
///
/// Use [`get_work_buffer_size`](Self::get_work_buffer_size) to query the
/// required transfer memory size, then
/// [`open_decoder`](Self::open_decoder) to create a decoder.
#[repr(transparent)]
pub struct HwopusService(Session);

impl HwopusService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }

    /// Gets the required work buffer size for single-stream decoding.
    ///
    /// The returned size should be page-aligned (rounded up to 0x1000) before
    /// creating transfer memory.
    #[inline]
    pub fn get_work_buffer_size(
        &self,
        sample_rate: i32,
        channel_count: i32,
    ) -> Result<u32, DispatchError> {
        cmif::get_work_buffer_size(&self.0, sample_rate, channel_count)
    }

    /// Gets the required work buffer size for multi-stream decoding. \[3.0.0+\]
    ///
    /// The returned size should be page-aligned (rounded up to 0x1000) before
    /// creating transfer memory.
    #[inline]
    pub fn get_work_buffer_size_multistream(
        &self,
        state: &HwopusMultistreamState,
    ) -> Result<u32, DispatchError> {
        cmif::get_work_buffer_size_for_multi_stream(&self.0, state)
    }

    /// Opens a single-stream hardware Opus decoder.
    ///
    /// `tmem_handle` is the raw handle of a transfer memory object created by
    /// the caller with at least `tmem_size` bytes (page-aligned). The caller
    /// retains ownership of the transfer memory and must close it after the
    /// decoder is closed.
    pub fn open_decoder(
        &self,
        sample_rate: i32,
        channel_count: i32,
        tmem_handle: u32,
        tmem_size: u32,
    ) -> Result<HwopusDecoder, OpenDecoderError> {
        let raw_handle = cmif::open_hardware_opus_decoder(
            &self.0,
            sample_rate,
            channel_count,
            tmem_handle,
            tmem_size,
        )?;

        // SAFETY: `raw_handle` is a fresh kernel session handle returned by
        // `OpenHardwareOpusDecoder`; ownership transfers to the new `Session`.
        let session = unsafe { SessionHandle::from_raw(raw_handle) };
        Ok(HwopusDecoder {
            service: Session::from_handle(session, 0),
            multistream: false,
        })
    }

    /// Opens a multi-stream hardware Opus decoder. \[3.0.0+\]
    ///
    /// `tmem_handle` is the raw handle of a transfer memory object created by
    /// the caller with at least `tmem_size` bytes (page-aligned). The caller
    /// retains ownership of the transfer memory and must close it after the
    /// decoder is closed.
    pub fn open_decoder_multistream(
        &self,
        state: &HwopusMultistreamState,
        tmem_handle: u32,
        tmem_size: u32,
    ) -> Result<HwopusDecoder, OpenDecoderError> {
        let raw_handle = cmif::open_hardware_opus_decoder_for_multi_stream(
            &self.0,
            state,
            tmem_handle,
            tmem_size,
        )?;

        // SAFETY: `raw_handle` is a fresh kernel session handle returned by
        // `OpenHardwareOpusDecoderForMultiStream`; ownership transfers to the
        // new `Session`.
        let session = unsafe { SessionHandle::from_raw(raw_handle) };
        Ok(HwopusDecoder {
            service: Session::from_handle(session, 0),
            multistream: true,
        })
    }
}

/// Hardware Opus decoder session wrapper.
///
/// Obtained via [`HwopusService::open_decoder`] or
/// [`HwopusService::open_decoder_multistream`]. Owns its own independent
/// session handle.
pub struct HwopusDecoder {
    service: Session,
    multistream: bool,
}

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for HwopusDecoder {}
unsafe impl Sync for HwopusDecoder {}

impl HwopusDecoder {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.service.handle()
    }

    /// Returns whether this decoder was opened in multi-stream mode.
    #[inline]
    pub fn is_multistream(&self) -> bool {
        self.multistream
    }

    /// Decodes interleaved Opus data (pre-4.0.0).
    ///
    /// `opusin` contains the Opus packet data (with [`HwopusHeader`] prefix).
    /// `pcmbuf` receives the decoded PCM samples.
    ///
    /// On 4.0.0+ use [`decode_interleaved_with_perf`](Self::decode_interleaved_with_perf).
    #[inline]
    pub fn decode_interleaved_legacy(
        &self,
        opusin: &[u8],
        pcmbuf: &mut [i16],
    ) -> Result<DecodeResult, DispatchError> {
        let cmd_id = if self.multistream {
            proto::DECODE_INTERLEAVED_MULTI_STREAM
        } else {
            proto::DECODE_INTERLEAVED
        };
        cmif::decode_interleaved_legacy(&self.service, cmd_id, opusin, pcmbuf)
    }

    /// Decodes interleaved Opus data with performance output (4.0.0+).
    ///
    /// `opusin` contains the Opus packet data (with [`HwopusHeader`] prefix).
    /// `pcmbuf` receives the decoded PCM samples.
    ///
    /// On 6.0.0+ use [`decode_interleaved`](Self::decode_interleaved).
    /// On pre-4.0.0 use [`decode_interleaved_legacy`](Self::decode_interleaved_legacy).
    #[inline]
    pub fn decode_interleaved_with_perf(
        &self,
        opusin: &[u8],
        pcmbuf: &mut [i16],
    ) -> Result<DecodeResultWithPerf, DispatchError> {
        let cmd_id = if self.multistream {
            proto::DECODE_INTERLEAVED_WITH_PERF_MULTI_STREAM
        } else {
            proto::DECODE_INTERLEAVED_WITH_PERF
        };
        cmif::decode_interleaved_with_perf(&self.service, cmd_id, opusin, pcmbuf)
    }

    /// Decodes interleaved Opus data with performance output and optional
    /// context reset (6.0.0+).
    ///
    /// `opusin` contains the Opus packet data (with [`HwopusHeader`] prefix).
    /// `pcmbuf` receives the decoded PCM samples. `reset_context` resets the
    /// decoder state before decoding.
    ///
    /// On pre-6.0.0 use [`decode_interleaved_with_perf`](Self::decode_interleaved_with_perf)
    /// or [`decode_interleaved_legacy`](Self::decode_interleaved_legacy).
    #[inline]
    pub fn decode_interleaved(
        &self,
        reset_context: bool,
        opusin: &[u8],
        pcmbuf: &mut [i16],
    ) -> Result<DecodeResultWithPerf, DispatchError> {
        let cmd_id = if self.multistream {
            proto::DECODE_INTERLEAVED_EX_MULTI_STREAM
        } else {
            proto::DECODE_INTERLEAVED_EX
        };
        cmif::decode_interleaved_ex(&self.service, cmd_id, reset_context, opusin, pcmbuf)
    }
}

/// Connects to the `hwopus` manager service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<HwopusService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(HwopusService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get hwopus service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
