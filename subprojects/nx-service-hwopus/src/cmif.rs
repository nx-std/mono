//! CMIF protocol operations for the hardware Opus service.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Session,
};
use zerocopy::IntoBytes as _;

use crate::{
    proto,
    types::{
        DecodeResult,
        DecodeResultWithPerf,
        HwopusMultistreamState,
        OpenDecoderIn,
    },
};

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
        _pad: 0,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::OPEN_HARDWARE_OPUS_DECODER)
        .in_raw(input.as_bytes())
        .in_handle(tmem_handle)
        .send(&mut buf)
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

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::GET_WORK_BUFFER_SIZE)
        .in_raw(val.as_bytes())
        .out_size(size_of::<u32>())
        .send(&mut buf)?;

    Ok(*result.value::<u32>())
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

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::OPEN_HARDWARE_OPUS_DECODER_FOR_MULTI_STREAM)
        .in_raw(size_val.as_bytes())
        .in_buffer(state.as_bytes(), BufferAttr::HIPC_POINTER)
        .in_handle(tmem_handle)
        .send(&mut buf)
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
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::GET_WORK_BUFFER_SIZE_FOR_MULTI_STREAM)
        .out_size(size_of::<u32>())
        .in_buffer(state.as_bytes(), BufferAttr::HIPC_POINTER)
        .send(&mut buf)?;

    Ok(*result.value::<u32>())
}

/// Decodes interleaved Opus data (pre-4.0.0).
///
/// `cmd_id` selects single-stream (0) or multi-stream (2).
pub(crate) fn decode_interleaved_legacy(
    service: &Session,
    cmd_id: u32,
    opusin: &[u8],
    pcmbuf: &mut [i16],
) -> Result<DecodeResult, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<DecodeResult>())
        .in_buffer(opusin, BufferAttr::HIPC_MAP_ALIAS)
        .out_buffer(pcmbuf.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut buf)?;

    Ok(*result.value::<DecodeResult>())
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
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<DecodeResultWithPerf>())
        .in_buffer(opusin, BufferAttr::HIPC_MAP_ALIAS)
        .out_buffer(
            pcmbuf.as_mut_bytes(),
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE),
        )
        .send(&mut buf)?;

    Ok(*result.value::<DecodeResultWithPerf>())
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

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(reset_flag.as_bytes())
        .out_size(size_of::<DecodeResultWithPerf>())
        .in_buffer(opusin, BufferAttr::HIPC_MAP_ALIAS)
        .out_buffer(
            pcmbuf.as_mut_bytes(),
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE),
        )
        .send(&mut buf)?;

    Ok(*result.value::<DecodeResultWithPerf>())
}

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
