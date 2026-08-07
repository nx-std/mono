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

use crate::{
    proto,
    types::{
        DecodeJpegIn,
        ScreenShotDecodeOption,
        ShrinkJpegExIn,
    },
};

/// Decodes a JPEG buffer into RGBA8.
pub fn decode_jpeg(
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

/// Shrinks a JPEG's dimensions by 2 with auto-quality selection.
pub fn shrink_jpeg(
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

    let result_size = *resp.payload;

    Ok(result_size)
}

/// Shrinks a JPEG with explicit target dimensions and quality.
pub fn shrink_jpeg_ex(
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

    let result_size = *resp.payload;

    Ok(result_size)
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
