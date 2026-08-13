//! CMIF protocol operations for the screenshot control service.

use nx_service_vi::ViLayerStack;
use nx_sf::{
    cmif,
    hipc::{
        BufferMode,
        OutputBuffer,
    },
    service::BorrowedSessionHandle,
};
use static_assertions::const_assert_eq;

use super::proto;

/// Wire-layout input for [`capture_raw_image_with_timeout`] (cmd 2).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct CaptureRawImageIn {
    /// Layer stack the capture is taken from.
    layer_stack: ViLayerStack,
    /// Padding the wire form carries after the layer stack.
    _pad: u32,
    /// Width of the captured image, in pixels.
    width: u64,
    /// Height of the captured image, in pixels.
    height: u64,
    /// Number of buffers the output holds.
    buffer_count: i64,
    /// Index of the buffer to capture into.
    buffer_index: i64,
    /// Capture timeout, in nanoseconds.
    timeout: i64,
}

const_assert_eq!(size_of::<CaptureRawImageIn>(), 0x30);

/// Captures a raw RGBA8 screenshot with a timeout.
#[expect(
    clippy::too_many_arguments,
    reason = "the parameters are the fields of CaptureRawImageIn plus the output buffer; grouping them into \
              a struct would put the wire layout behind a second type the caller has to build first"
)]
pub(crate) fn capture_raw_image_with_timeout(
    session: BorrowedSessionHandle<'_>,
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

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::CAPTURE_RAW_IMAGE_WITH_TIMEOUT)
        .with_data_value(&input)
        .add_output_buffer(OutputBuffer::new(out_image, BufferMode::NonSecure))
        .build();
    req.send(&mut buf, session)
        .map_err(CaptureRawImageError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(CaptureRawImageError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`capture_raw_image_with_timeout`].
#[derive(Debug, thiserror::Error)]
pub enum CaptureRawImageError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Wire-layout input for [`open_raw_screen_shot_read_stream`] (cmd 1201) and
/// [`capture_jpeg_screen_shot`] (cmd 1204).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct LayerStackTimeoutIn {
    /// Layer stack the capture is taken from.
    layer_stack: ViLayerStack,
    /// Padding the wire form carries after the layer stack.
    _pad: u32,
    /// Capture timeout, in nanoseconds.
    timeout: i64,
}

const_assert_eq!(size_of::<LayerStackTimeoutIn>(), 0x10);

/// Wire-layout output for [`open_raw_screen_shot_read_stream`] (cmd 1201).
#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
struct OpenReadStreamOut {
    /// Size of the captured raw image, in bytes.
    size: u64,
    /// Width of the captured raw image, in pixels.
    width: u64,
    /// Height of the captured raw image, in pixels.
    height: u64,
}

const_assert_eq!(size_of::<OpenReadStreamOut>(), 0x18);

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
pub(crate) fn open_raw_screen_shot_read_stream(
    session: BorrowedSessionHandle<'_>,
    layer_stack: ViLayerStack,
    timeout: i64,
) -> Result<ReadStreamInfo, OpenReadStreamError> {
    let input = LayerStackTimeoutIn {
        layer_stack,
        _pad: 0,
        timeout,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::OPEN_RAW_SCREEN_SHOT_READ_STREAM)
        .with_data_value(&input)
        .build();
    req.send(&mut buf, session)
        .map_err(OpenReadStreamError::SendRequest)?;

    let resp = cmif::parse_response::<&OpenReadStreamOut>(&buf)
        .map_err(OpenReadStreamError::ParseResponse)?;

    let out = *resp.payload;

    Ok(ReadStreamInfo {
        size: out.size,
        width: out.width,
        height: out.height,
    })
}

/// Error returned by [`open_raw_screen_shot_read_stream`].
#[derive(Debug, thiserror::Error)]
pub enum OpenReadStreamError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Closes a raw screenshot read stream.
pub(crate) fn close_raw_screen_shot_read_stream(
    session: BorrowedSessionHandle<'_>,
) -> Result<(), CloseReadStreamError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::CLOSE_RAW_SCREEN_SHOT_READ_STREAM).build();
    req.send(&mut buf, session)
        .map_err(CloseReadStreamError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(CloseReadStreamError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`close_raw_screen_shot_read_stream`].
#[derive(Debug, thiserror::Error)]
pub enum CloseReadStreamError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Reads from a raw screenshot read stream.
///
/// Returns the number of bytes written to the output buffer.
pub(crate) fn read_raw_screen_shot_read_stream(
    session: BorrowedSessionHandle<'_>,
    offset: u64,
    out_buf: &mut [u8],
) -> Result<u64, ReadStreamError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::READ_RAW_SCREEN_SHOT_READ_STREAM)
        .with_data_value(&offset)
        .add_output_buffer(OutputBuffer::new(out_buf, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(ReadStreamError::SendRequest)?;

    let resp = cmif::parse_response::<&u64>(&buf).map_err(ReadStreamError::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by [`read_raw_screen_shot_read_stream`].
#[derive(Debug, thiserror::Error)]
pub enum ReadStreamError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Captures a JPEG screenshot.
///
/// Returns the size of the captured JPEG in the output buffer.
pub(crate) fn capture_jpeg_screen_shot(
    session: BorrowedSessionHandle<'_>,
    layer_stack: ViLayerStack,
    timeout: i64,
    out_jpeg: &mut [u8],
) -> Result<u64, CaptureJpegError> {
    let input = LayerStackTimeoutIn {
        layer_stack,
        _pad: 0,
        timeout,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::CAPTURE_JPEG_SCREEN_SHOT)
        .with_data_value(&input)
        .add_output_buffer(OutputBuffer::new(out_jpeg, BufferMode::NonSecure))
        .build();
    req.send(&mut buf, session)
        .map_err(CaptureJpegError::SendRequest)?;

    let resp = cmif::parse_response::<&u64>(&buf).map_err(CaptureJpegError::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by [`capture_jpeg_screen_shot`].
#[derive(Debug, thiserror::Error)]
pub enum CaptureJpegError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
