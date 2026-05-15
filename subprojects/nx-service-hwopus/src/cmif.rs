//! CMIF protocol operations for the hardware Opus service.

use core::{mem::size_of, ptr};

use nx_sf::service::{BufferAttr, DispatchError, Session};

use crate::{
    proto,
    types::{DecodeResult, DecodeResultWithPerf, HwopusMultistreamState, OpenDecoderIn},
};

// ---------------------------------------------------------------------------
// Manager commands (IHardwareOpusDecoderManager)
// ---------------------------------------------------------------------------

/// Opens a single-stream hardware Opus decoder (cmd 0).
///
/// Returns the decoder session handle (move handle).
pub(crate) fn open_hardware_opus_decoder(
    service: &Session,
    sample_rate: i32,
    channel_count: i32,
    tmem_handle: u32,
    tmem_size: u32,
) -> Result<u32, OpenDecoderError> {
    let input = OpenDecoderIn {
        val: (sample_rate as u64) | ((channel_count as u64) << 32),
        size: tmem_size,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<OpenDecoderIn>())
    };
    let result = service
        .dispatch(proto::OPEN_HARDWARE_OPUS_DECODER)
        .in_raw(in_bytes)
        .in_handle(tmem_handle)
        .send()
        .map_err(OpenDecoderError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(OpenDecoderError::MissingHandle);
    }
    Ok(result.move_handles[0])
}

/// Gets the required work buffer size for single-stream decoding (cmd 1).
pub(crate) fn get_work_buffer_size(
    service: &Session,
    sample_rate: i32,
    channel_count: i32,
) -> Result<u32, DispatchError> {
    let val: u64 = (sample_rate as u64) | ((channel_count as u64) << 32);

    // SAFETY: `val` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const val).cast::<u8>(), size_of::<u64>()) };
    let result = service
        .dispatch(proto::GET_WORK_BUFFER_SIZE)
        .in_raw(in_bytes)
        .out_size(size_of::<u32>())
        .send()?;

    // SAFETY: response payload is at least size_of::<u32>().
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}

/// Opens a multi-stream hardware Opus decoder (cmd 2). [3.0.0+]
///
/// Returns the decoder session handle (move handle).
pub(crate) fn open_hardware_opus_decoder_for_multi_stream(
    service: &Session,
    state: &HwopusMultistreamState,
    tmem_handle: u32,
    tmem_size: u32,
) -> Result<u32, OpenDecoderError> {
    let size_val: u64 = tmem_size as u64;

    // SAFETY: `size_val` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const size_val).cast::<u8>(), size_of::<u64>())
    };
    // SAFETY: `state` is a valid reference; viewing its bytes as a slice for
    // the IN pointer buffer is sound, and the slice borrows `state`.
    let state_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const *state).cast::<u8>(),
            size_of::<HwopusMultistreamState>(),
        )
    };
    let result = service
        .dispatch(proto::OPEN_HARDWARE_OPUS_DECODER_FOR_MULTI_STREAM)
        .in_raw(in_bytes)
        .in_buffer(state_bytes, BufferAttr::HIPC_POINTER)
        .in_handle(tmem_handle)
        .send()
        .map_err(OpenDecoderError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(OpenDecoderError::MissingHandle);
    }
    Ok(result.move_handles[0])
}

/// Gets the required work buffer size for multi-stream decoding (cmd 3). [3.0.0+]
pub(crate) fn get_work_buffer_size_for_multi_stream(
    service: &Session,
    state: &HwopusMultistreamState,
) -> Result<u32, DispatchError> {
    // SAFETY: `state` is a valid reference; viewing its bytes as a slice for
    // the IN pointer buffer is sound, and the slice borrows `state`.
    let state_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const *state).cast::<u8>(),
            size_of::<HwopusMultistreamState>(),
        )
    };
    let result = service
        .dispatch(proto::GET_WORK_BUFFER_SIZE_FOR_MULTI_STREAM)
        .out_size(size_of::<u32>())
        .in_buffer(state_bytes, BufferAttr::HIPC_POINTER)
        .send()?;

    // SAFETY: response payload is at least size_of::<u32>().
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}

// ---------------------------------------------------------------------------
// Decoder commands (IHardwareOpusDecoder)
// ---------------------------------------------------------------------------

/// Decodes interleaved Opus data (pre-4.0.0).
///
/// `cmd_id` selects single-stream (0) or multi-stream (2).
pub(crate) fn decode_interleaved_legacy(
    service: &Session,
    cmd_id: u32,
    opusin: &[u8],
    pcmbuf: &mut [i16],
) -> Result<DecodeResult, DispatchError> {
    // SAFETY: `pcmbuf` is a valid `&mut [i16]`; viewing it as mutable bytes
    // for the OUT buffer is sound, and the slice borrows `pcmbuf`.
    let pcm_bytes = unsafe {
        core::slice::from_raw_parts_mut(pcmbuf.as_mut_ptr().cast::<u8>(), size_of_val(pcmbuf))
    };
    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<DecodeResult>())
        .in_buffer(opusin, BufferAttr::HIPC_MAP_ALIAS)
        .out_buffer(pcm_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;

    // SAFETY: response payload is at least size_of::<DecodeResult>().
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<DecodeResult>()) })
}

/// Decodes interleaved Opus data with performance output (4.0.0+).
///
/// `cmd_id` selects single-stream (4) or multi-stream (5).
pub(crate) fn decode_interleaved_with_perf(
    service: &Session,
    cmd_id: u32,
    opusin: &[u8],
    pcmbuf: &mut [i16],
) -> Result<DecodeResultWithPerf, DispatchError> {
    // SAFETY: `pcmbuf` is a valid `&mut [i16]`; viewing it as mutable bytes
    // for the OUT buffer is sound, and the slice borrows `pcmbuf`.
    let pcm_bytes = unsafe {
        core::slice::from_raw_parts_mut(pcmbuf.as_mut_ptr().cast::<u8>(), size_of_val(pcmbuf))
    };
    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<DecodeResultWithPerf>())
        .in_buffer(opusin, BufferAttr::HIPC_MAP_ALIAS)
        .out_buffer(
            pcm_bytes,
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE),
        )
        .send()?;

    // SAFETY: response payload is at least size_of::<DecodeResultWithPerf>().
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<DecodeResultWithPerf>()) })
}

/// Decodes interleaved Opus data with performance output and context reset
/// (6.0.0+).
///
/// `cmd_id` selects single-stream (6) or multi-stream (7).
pub(crate) fn decode_interleaved_ex(
    service: &Session,
    cmd_id: u32,
    reset_context: bool,
    opusin: &[u8],
    pcmbuf: &mut [i16],
) -> Result<DecodeResultWithPerf, DispatchError> {
    let reset_flag: u8 = reset_context as u8;

    // SAFETY: `reset_flag` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const reset_flag).cast::<u8>(), size_of::<u8>())
    };
    // SAFETY: `pcmbuf` is a valid `&mut [i16]`; viewing it as mutable bytes
    // for the OUT buffer is sound, and the slice borrows `pcmbuf`.
    let pcm_bytes = unsafe {
        core::slice::from_raw_parts_mut(pcmbuf.as_mut_ptr().cast::<u8>(), size_of_val(pcmbuf))
    };
    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<DecodeResultWithPerf>())
        .in_buffer(opusin, BufferAttr::HIPC_MAP_ALIAS)
        .out_buffer(
            pcm_bytes,
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE),
        )
        .send()?;

    // SAFETY: response payload is at least size_of::<DecodeResultWithPerf>().
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<DecodeResultWithPerf>()) })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn size_of_val<T>(slice: &[T]) -> usize {
    core::mem::size_of_val(slice)
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned by decoder-open operations.
#[derive(Debug, thiserror::Error)]
pub enum OpenDecoderError {
    /// IPC dispatch failed.
    #[error("failed to dispatch OpenHardwareOpusDecoder")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected session handle.
    #[error("OpenHardwareOpusDecoder response missing move handle")]
    MissingHandle,
}
