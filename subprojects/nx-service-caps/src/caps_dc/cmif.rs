//! CMIF protocol operations for the JPEG decoder service.

use nx_sf::{
    cmif,
    hipc::{
        BufferMode,
        InputBuffer,
        OutputBuffer,
    },
    service::BorrowedSessionHandle,
};
use static_assertions::const_assert_eq;

use super::proto;
use crate::screenshot::ScreenShotDecodeOption;

/// Wire-layout input for [`decode_jpeg`] (cmd 3001) and [`shrink_jpeg`] (cmd 4001).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct DecodeJpegIn {
    /// Width of the source image, in pixels.
    width: u32,
    /// Height of the source image, in pixels.
    height: u32,
    /// Decoder behaviour flags.
    opts: ScreenShotDecodeOption,
}

const_assert_eq!(size_of::<DecodeJpegIn>(), 0x28);

/// Decodes a JPEG buffer into RGBA8.
pub(crate) fn decode_jpeg(
    session: BorrowedSessionHandle<'_>,
    width: u32,
    height: u32,
    opts: &ScreenShotDecodeOption,
    jpeg: &[u8],
    out_image: &mut [u8],
) -> Result<(), DecodeJpegError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let input = DecodeJpegIn {
        width,
        height,
        opts: *opts,
    };
    let req = cmif::CmifRequestBuilder::new(proto::DECODE_JPEG)
        .with_data_value(&input)
        .add_input_buffer(InputBuffer::new(jpeg, BufferMode::Normal))
        .add_output_buffer(OutputBuffer::new(out_image, BufferMode::NonSecure))
        .build();
    req.send(&mut buf, session)
        .map_err(DecodeJpegError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DecodeJpegError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`decode_jpeg`].
#[derive(Debug, thiserror::Error)]
pub enum DecodeJpegError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Shrinks a JPEG's dimensions by 2 with auto-quality selection.
pub(crate) fn shrink_jpeg(
    session: BorrowedSessionHandle<'_>,
    width: u32,
    height: u32,
    opts: &ScreenShotDecodeOption,
    jpeg: &[u8],
    out_jpeg: &mut [u8],
) -> Result<u64, ShrinkJpegError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let input = DecodeJpegIn {
        width,
        height,
        opts: *opts,
    };
    let req = cmif::CmifRequestBuilder::new(proto::SHRINK_JPEG)
        .with_data_value(&input)
        .add_input_buffer(InputBuffer::new(jpeg, BufferMode::Normal))
        .add_output_buffer(OutputBuffer::new(out_jpeg, BufferMode::NonSecure))
        .build();
    req.send(&mut buf, session)
        .map_err(ShrinkJpegError::SendRequest)?;

    let resp = cmif::parse_response::<&u64>(&buf).map_err(ShrinkJpegError::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by [`shrink_jpeg`].
#[derive(Debug, thiserror::Error)]
pub enum ShrinkJpegError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Wire-layout input for [`shrink_jpeg_ex`] (cmd 4002).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct ShrinkJpegExIn {
    /// Target width of the shrunk image, in pixels.
    scaled_width: u32,
    /// Target height of the shrunk image, in pixels.
    scaled_height: u32,
    /// JPEG compression quality, 0-100.
    jpeg_quality: u32,
    /// Padding the wire form carries before the options.
    _pad: [u8; 4],
    /// Decoder behaviour flags.
    opts: ScreenShotDecodeOption,
}

const_assert_eq!(size_of::<ShrinkJpegExIn>(), 0x30);

/// Shrinks a JPEG with explicit target dimensions and quality.
pub(crate) fn shrink_jpeg_ex(
    session: BorrowedSessionHandle<'_>,
    scaled_width: u32,
    scaled_height: u32,
    jpeg_quality: u32,
    opts: &ScreenShotDecodeOption,
    jpeg: &[u8],
    out_jpeg: &mut [u8],
) -> Result<u64, ShrinkJpegExError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let input = ShrinkJpegExIn {
        scaled_width,
        scaled_height,
        jpeg_quality,
        _pad: [0; 4],
        opts: *opts,
    };
    let req = cmif::CmifRequestBuilder::new(proto::SHRINK_JPEG_EX)
        .with_data_value(&input)
        .add_input_buffer(InputBuffer::new(jpeg, BufferMode::Normal))
        .add_output_buffer(OutputBuffer::new(out_jpeg, BufferMode::NonSecure))
        .build();
    req.send(&mut buf, session)
        .map_err(ShrinkJpegExError::SendRequest)?;

    let resp = cmif::parse_response::<&u64>(&buf).map_err(ShrinkJpegExError::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by [`shrink_jpeg_ex`].
#[derive(Debug, thiserror::Error)]
pub enum ShrinkJpegExError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
