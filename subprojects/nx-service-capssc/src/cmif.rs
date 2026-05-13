//! CMIF protocol operations for the screenshot control service.

use core::ptr;

use nx_service_vi::ViLayerStack;
use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::ipc::{self, Handle as SessionHandle};

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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::CAPTURE_RAW_IMAGE_WITH_TIMEOUT)
        .data_size(size_of::<CaptureRawImageIn>())
        .out_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    let input = CaptureRawImageIn {
        layer_stack,
        _pad: 0,
        width,
        height,
        buffer_count,
        buffer_index,
        timeout,
    };

    // SAFETY: req.data points to valid payload area with space for CaptureRawImageIn.
    unsafe {
        ptr::write_unaligned(
            req.data.as_ptr().cast::<CaptureRawImageIn>().cast_mut(),
            input,
        );
    }

    req.add_out_buffer(
        out_image.as_mut_ptr(),
        out_image.len(),
        BufferMode::NonSecure,
    );

    ipc::send_sync_request(session).map_err(CaptureRawImageError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(CaptureRawImageError::ParseResponse)?;

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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::OPEN_RAW_SCREEN_SHOT_READ_STREAM)
        .data_size(size_of::<LayerStackTimeoutIn>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    let input = LayerStackTimeoutIn {
        layer_stack,
        _pad: 0,
        timeout,
    };

    // SAFETY: req.data points to valid payload area with space for LayerStackTimeoutIn.
    unsafe {
        ptr::write_unaligned(
            req.data.as_ptr().cast::<LayerStackTimeoutIn>().cast_mut(),
            input,
        );
    }

    ipc::send_sync_request(session).map_err(OpenReadStreamError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<OpenReadStreamOut>()) }
        .map_err(OpenReadStreamError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for OpenReadStreamOut.
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::CLOSE_RAW_SCREEN_SHOT_READ_STREAM).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(CloseReadStreamError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(CloseReadStreamError::ParseResponse)?;

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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::READ_RAW_SCREEN_SHOT_READ_STREAM)
        .data_size(size_of::<u64>())
        .out_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u64.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), offset);
    }

    req.add_out_buffer(out_buf.as_mut_ptr(), out_buf.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(ReadStreamError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u64>()) }
        .map_err(ReadStreamError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u64.
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::CAPTURE_JPEG_SCREEN_SHOT)
        .data_size(size_of::<LayerStackTimeoutIn>())
        .out_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    let input = LayerStackTimeoutIn {
        layer_stack,
        _pad: 0,
        timeout,
    };

    // SAFETY: req.data points to valid payload area with space for LayerStackTimeoutIn.
    unsafe {
        ptr::write_unaligned(
            req.data.as_ptr().cast::<LayerStackTimeoutIn>().cast_mut(),
            input,
        );
    }

    req.add_out_buffer(out_jpeg.as_mut_ptr(), out_jpeg.len(), BufferMode::NonSecure);

    ipc::send_sync_request(session).map_err(CaptureJpegError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u64>()) }
        .map_err(CaptureJpegError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u64.
    let jpeg_size = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(jpeg_size)
}

/// Error returned by [`capture_raw_image_with_timeout`].
#[derive(Debug, thiserror::Error)]
pub enum CaptureRawImageError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`open_raw_screen_shot_read_stream`].
#[derive(Debug, thiserror::Error)]
pub enum OpenReadStreamError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`close_raw_screen_shot_read_stream`].
#[derive(Debug, thiserror::Error)]
pub enum CloseReadStreamError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`read_raw_screen_shot_read_stream`].
#[derive(Debug, thiserror::Error)]
pub enum ReadStreamError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`capture_jpeg_screen_shot`].
#[derive(Debug, thiserror::Error)]
pub enum CaptureJpegError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
