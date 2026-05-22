//! CMIF protocol operations for the screenshot control service.

use core::ptr;

use nx_service_vi::ViLayerStack;
use nx_sf::{
    cmif,
    hipc::BufferMode,
    ipc::{self, Handle as SessionHandle},
};

use crate::{
    proto,
    types::{CaptureRawImageIn, LayerStackTimeoutIn, OpenReadStreamOut},
};

/// Captures a raw RGBA8 screenshot with a timeout.
#[allow(clippy::too_many_arguments)]
pub fn capture_raw_image_with_timeout(
    session: SessionHandle,
    layer_stack: ViLayerStack,
    width: u64,
    height: u64,
    buffer_count: i64,
    buffer_index: i64,
    timeout: i64,
    out_image: &mut [u8],
) -> Result<(), CaptureRawImageError> {
    let input = CaptureRawImageIn {
        layer_stack,
        _pad: 0,
        width,
        height,
        buffer_count,
        buffer_index,
        timeout,
    };

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.write_to`
    // completes; viewing its bytes as a slice is sound.
    let data = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<CaptureRawImageIn>(),
        )
    };
    let req = cmif::CmifRequestBuilder::new(proto::CAPTURE_RAW_IMAGE_WITH_TIMEOUT)
        .with_data(data)
        .add_output_buffer_raw(
            out_image.as_mut_ptr(),
            out_image.len(),
            BufferMode::NonSecure,
        )
        .build();
    req.write_to(&mut buf)
        .map_err(CaptureRawImageError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(CaptureRawImageError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(CaptureRawImageError::ParseResponse)?;

    Ok(())
}

/// Raw screenshot stream metadata returned by [`open_raw_screen_shot_read_stream`].
pub struct ReadStreamInfo {
    /// Size of the captured raw image in bytes (always 0x384000 = 1280*720*4).
    pub size: u64,
    /// Width of the captured raw image (always 1280).
    pub width: u64,
    /// Height of the captured raw image (always 720).
    pub height: u64,
}

/// Opens a raw screenshot read stream.
pub fn open_raw_screen_shot_read_stream(
    session: SessionHandle,
    layer_stack: ViLayerStack,
    timeout: i64,
) -> Result<ReadStreamInfo, OpenReadStreamError> {
    let input = LayerStackTimeoutIn {
        layer_stack,
        _pad: 0,
        timeout,
    };

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.write_to`
    // completes; viewing its bytes as a slice is sound.
    let data = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<LayerStackTimeoutIn>(),
        )
    };
    let req = cmif::CmifRequestBuilder::new(proto::OPEN_RAW_SCREEN_SHOT_READ_STREAM)
        .with_data(data)
        .build();
    req.write_to(&mut buf)
        .map_err(OpenReadStreamError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(OpenReadStreamError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<OpenReadStreamOut>())
        .map_err(OpenReadStreamError::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<OpenReadStreamOut>()` bytes.
    let out = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<OpenReadStreamOut>()) };

    Ok(ReadStreamInfo {
        size: out.size,
        width: out.width,
        height: out.height,
    })
}

/// Closes a raw screenshot read stream.
pub fn close_raw_screen_shot_read_stream(
    session: SessionHandle,
) -> Result<(), CloseReadStreamError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::CLOSE_RAW_SCREEN_SHOT_READ_STREAM).build();
    req.write_to(&mut buf)
        .map_err(CloseReadStreamError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(CloseReadStreamError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(CloseReadStreamError::ParseResponse)?;

    Ok(())
}

/// Reads from a raw screenshot read stream.
///
/// Returns the number of bytes written to the output buffer.
pub fn read_raw_screen_shot_read_stream(
    session: SessionHandle,
    offset: u64,
    out_buf: &mut [u8],
) -> Result<u64, ReadStreamError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::READ_RAW_SCREEN_SHOT_READ_STREAM)
        .with_data_value(&offset)
        .add_output_buffer_raw(out_buf.as_mut_ptr(), out_buf.len(), BufferMode::Normal)
        .build();
    req.write_to(&mut buf)
        .map_err(ReadStreamError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(ReadStreamError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u64>())
        .map_err(ReadStreamError::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<u64>()` bytes.
    let bytes_read = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(bytes_read)
}

/// Captures a JPEG screenshot.
///
/// Returns the size of the captured JPEG in the output buffer.
pub fn capture_jpeg_screen_shot(
    session: SessionHandle,
    layer_stack: ViLayerStack,
    timeout: i64,
    out_jpeg: &mut [u8],
) -> Result<u64, CaptureJpegError> {
    let input = LayerStackTimeoutIn {
        layer_stack,
        _pad: 0,
        timeout,
    };

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.write_to`
    // completes; viewing its bytes as a slice is sound.
    let data = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<LayerStackTimeoutIn>(),
        )
    };
    let req = cmif::CmifRequestBuilder::new(proto::CAPTURE_JPEG_SCREEN_SHOT)
        .with_data(data)
        .add_output_buffer_raw(out_jpeg.as_mut_ptr(), out_jpeg.len(), BufferMode::NonSecure)
        .build();
    req.write_to(&mut buf)
        .map_err(CaptureJpegError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(CaptureJpegError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u64>())
        .map_err(CaptureJpegError::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<u64>()` bytes.
    let jpeg_size = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(jpeg_size)
}

/// Error returned by [`capture_raw_image_with_timeout`].
#[derive(Debug, thiserror::Error)]
pub enum CaptureRawImageError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Error returned by [`open_raw_screen_shot_read_stream`].
#[derive(Debug, thiserror::Error)]
pub enum OpenReadStreamError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Error returned by [`close_raw_screen_shot_read_stream`].
#[derive(Debug, thiserror::Error)]
pub enum CloseReadStreamError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Error returned by [`read_raw_screen_shot_read_stream`].
#[derive(Debug, thiserror::Error)]
pub enum ReadStreamError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Error returned by [`capture_jpeg_screen_shot`].
#[derive(Debug, thiserror::Error)]
pub enum CaptureJpegError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}
