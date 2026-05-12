//! CMIF protocol operations for the JPEG decoder service.

use core::ptr;

use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::{
    proto,
    types::{DecodeJpegIn, ScreenShotDecodeOption, ShrinkJpegExIn},
};

/// Decodes a JPEG buffer into RGBA8.
pub fn decode_jpeg(
    session: SessionHandle,
    width: u32,
    height: u32,
    opts: &ScreenShotDecodeOption,
    jpeg: &[u8],
    out_image: &mut [u8],
) -> Result<(), DecodeJpegError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::DECODE_JPEG)
        .data_size(size_of::<DecodeJpegIn>())
        .in_buffers(1)
        .out_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    let input = DecodeJpegIn {
        width,
        height,
        opts: *opts,
    };

    // SAFETY: req.data points to valid payload area with space for DecodeJpegIn.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<DecodeJpegIn>().cast_mut(), input);
    }

    req.add_in_buffer(jpeg.as_ptr(), jpeg.len(), BufferMode::Normal);
    req.add_out_buffer(
        out_image.as_mut_ptr(),
        out_image.len(),
        BufferMode::NonSecure,
    );

    ipc::send_sync_request(session).map_err(DecodeJpegError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(DecodeJpegError::ParseResponse)?;

    Ok(())
}

/// Shrinks a JPEG's dimensions by 2 with auto-quality selection.
pub fn shrink_jpeg(
    session: SessionHandle,
    width: u32,
    height: u32,
    opts: &ScreenShotDecodeOption,
    jpeg: &[u8],
    out_jpeg: &mut [u8],
) -> Result<u64, ShrinkJpegError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SHRINK_JPEG)
        .data_size(size_of::<DecodeJpegIn>())
        .in_buffers(1)
        .out_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    let input = DecodeJpegIn {
        width,
        height,
        opts: *opts,
    };

    // SAFETY: req.data points to valid payload area with space for DecodeJpegIn.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<DecodeJpegIn>().cast_mut(), input);
    }

    req.add_in_buffer(jpeg.as_ptr(), jpeg.len(), BufferMode::Normal);
    req.add_out_buffer(out_jpeg.as_mut_ptr(), out_jpeg.len(), BufferMode::NonSecure);

    ipc::send_sync_request(session).map_err(ShrinkJpegError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u64>()) }
        .map_err(ShrinkJpegError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u64.
    let result_size = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(result_size)
}

/// Shrinks a JPEG with explicit target dimensions and quality.
pub fn shrink_jpeg_ex(
    session: SessionHandle,
    scaled_width: u32,
    scaled_height: u32,
    jpeg_quality: u32,
    opts: &ScreenShotDecodeOption,
    jpeg: &[u8],
    out_jpeg: &mut [u8],
) -> Result<u64, ShrinkJpegExError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SHRINK_JPEG_EX)
        .data_size(size_of::<ShrinkJpegExIn>())
        .in_buffers(1)
        .out_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    let input = ShrinkJpegExIn {
        scaled_width,
        scaled_height,
        jpeg_quality,
        _pad: [0; 4],
        opts: *opts,
    };

    // SAFETY: req.data points to valid payload area with space for ShrinkJpegExIn.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<ShrinkJpegExIn>().cast_mut(), input);
    }

    req.add_in_buffer(jpeg.as_ptr(), jpeg.len(), BufferMode::Normal);
    req.add_out_buffer(out_jpeg.as_mut_ptr(), out_jpeg.len(), BufferMode::NonSecure);

    ipc::send_sync_request(session).map_err(ShrinkJpegExError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u64>()) }
        .map_err(ShrinkJpegExError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u64.
    let result_size = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(result_size)
}

/// Error returned by [`decode_jpeg`].
#[derive(Debug, thiserror::Error)]
pub enum DecodeJpegError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`shrink_jpeg`].
#[derive(Debug, thiserror::Error)]
pub enum ShrinkJpegError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`shrink_jpeg_ex`].
#[derive(Debug, thiserror::Error)]
pub enum ShrinkJpegExError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
