//! CMIF protocol operations for the JPEG decoder service.

use core::{mem::size_of, ptr};

use nx_sf::{
    cmif,
    hipc::BufferMode,
    ipc::{self, Handle as SessionHandle},
};

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
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::DECODE_JPEG)
        .data_size(size_of::<DecodeJpegIn>())
        .add_in_buffer(jpeg.as_ptr(), jpeg.len(), BufferMode::Normal)
        .add_out_buffer(
            out_image.as_mut_ptr(),
            out_image.len(),
            BufferMode::NonSecure,
        )
        .build();
    req.write_to(&mut buf)
        .map_err(DecodeJpegError::BuildRequest)?;

    let input = DecodeJpegIn {
        width,
        height,
        opts: *opts,
    };

    // SAFETY: `req` is exactly `size_of::<DecodeJpegIn>()` bytes.
    unsafe {
        ptr::write_unaligned(
            buf.as_array_mut().as_mut_ptr().cast::<DecodeJpegIn>(),
            input,
        )
    };

    ipc::send_sync_request(&mut buf, session).map_err(DecodeJpegError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(DecodeJpegError::ParseResponse)?;

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
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::SHRINK_JPEG)
        .data_size(size_of::<DecodeJpegIn>())
        .add_in_buffer(jpeg.as_ptr(), jpeg.len(), BufferMode::Normal)
        .add_out_buffer(out_jpeg.as_mut_ptr(), out_jpeg.len(), BufferMode::NonSecure)
        .build();
    req.write_to(&mut buf)
        .map_err(ShrinkJpegError::BuildRequest)?;

    let input = DecodeJpegIn {
        width,
        height,
        opts: *opts,
    };

    // SAFETY: `req` is exactly `size_of::<DecodeJpegIn>()` bytes.
    unsafe {
        ptr::write_unaligned(
            buf.as_array_mut().as_mut_ptr().cast::<DecodeJpegIn>(),
            input,
        )
    };

    ipc::send_sync_request(&mut buf, session).map_err(ShrinkJpegError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u64>())
        .map_err(ShrinkJpegError::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<u64>()` bytes.
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
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::SHRINK_JPEG_EX)
        .data_size(size_of::<ShrinkJpegExIn>())
        .add_in_buffer(jpeg.as_ptr(), jpeg.len(), BufferMode::Normal)
        .add_out_buffer(out_jpeg.as_mut_ptr(), out_jpeg.len(), BufferMode::NonSecure)
        .build();
    req.write_to(&mut buf)
        .map_err(ShrinkJpegExError::BuildRequest)?;

    let input = ShrinkJpegExIn {
        scaled_width,
        scaled_height,
        jpeg_quality,
        _pad: [0; 4],
        opts: *opts,
    };

    // SAFETY: `req` is exactly `size_of::<ShrinkJpegExIn>()` bytes.
    unsafe {
        ptr::write_unaligned(
            buf.as_array_mut().as_mut_ptr().cast::<ShrinkJpegExIn>(),
            input,
        )
    };

    ipc::send_sync_request(&mut buf, session).map_err(ShrinkJpegExError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u64>())
        .map_err(ShrinkJpegExError::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<u64>()` bytes.
    let result_size = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(result_size)
}

/// Error returned by [`decode_jpeg`].
#[derive(Debug, thiserror::Error)]
pub enum DecodeJpegError {
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

/// Error returned by [`shrink_jpeg`].
#[derive(Debug, thiserror::Error)]
pub enum ShrinkJpegError {
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

/// Error returned by [`shrink_jpeg_ex`].
#[derive(Debug, thiserror::Error)]
pub enum ShrinkJpegExError {
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
