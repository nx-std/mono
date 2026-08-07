//! CMIF protocol operations for the mii image service.

use nx_sf::{
    cmif,
    hipc::{
        BufferMode,
        OutputBuffer,
    },
    service::BorrowedSessionHandle,
};

use crate::{
    proto,
    types::MiiimgImageAttribute,
};

/// Initializes the image database.
///
/// Sends a `u8` mode value and returns a `u8` result.
pub fn initialize(session: BorrowedSessionHandle<'_>, mode: u8) -> Result<u8, InitializeError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::INITIALIZE)
        .with_data_value(&mode)
        .build();
    req.send(&mut buf, session)
        .map_err(InitializeError::SendRequest)?;

    let resp = cmif::parse_response::<&u8>(&buf).map_err(InitializeError::ParseResponse)?;
    let out = *resp.payload;

    Ok(out)
}

/// Reloads the image database.
pub fn reload(session: BorrowedSessionHandle<'_>) -> Result<(), ReloadError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::RELOAD).build();
    req.send(&mut buf, session)
        .map_err(ReloadError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(ReloadError::ParseResponse)?;

    Ok(())
}

/// Gets the number of mii images in the database.
pub fn get_count(session: BorrowedSessionHandle<'_>) -> Result<i32, GetCountError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::GET_COUNT).build();
    req.send(&mut buf, session)
        .map_err(GetCountError::SendRequest)?;

    let resp = cmif::parse_response::<&i32>(&buf).map_err(GetCountError::ParseResponse)?;
    let count = *resp.payload;

    Ok(count)
}

/// Gets whether the image database is empty.
pub fn is_empty(session: BorrowedSessionHandle<'_>) -> Result<bool, IsEmptyError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::IS_EMPTY).build();
    req.send(&mut buf, session)
        .map_err(IsEmptyError::SendRequest)?;

    let resp = cmif::parse_response::<&u8>(&buf).map_err(IsEmptyError::ParseResponse)?;
    let val = *resp.payload;

    Ok(val & 1 != 0)
}

/// Gets whether the image database is full.
pub fn is_full(session: BorrowedSessionHandle<'_>) -> Result<bool, IsFullError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::IS_FULL).build();
    req.send(&mut buf, session)
        .map_err(IsFullError::SendRequest)?;

    let resp = cmif::parse_response::<&u8>(&buf).map_err(IsFullError::ParseResponse)?;
    let val = *resp.payload;

    Ok(val & 1 != 0)
}

/// Gets the image attribute for the specified index.
pub fn get_attribute(
    session: BorrowedSessionHandle<'_>,
    index: i32,
) -> Result<MiiimgImageAttribute, GetAttributeError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::GET_ATTRIBUTE)
        .with_data_value(&index)
        .build();
    req.send(&mut buf, session)
        .map_err(GetAttributeError::SendRequest)?;

    let resp = cmif::parse_response::<&MiiimgImageAttribute>(&buf)
        .map_err(GetAttributeError::ParseResponse)?;
    let attr = *resp.payload;

    Ok(attr)
}

/// Loads the image data (raw RGBA8) for the specified image ID.
pub fn load_image(
    session: BorrowedSessionHandle<'_>,
    id: crate::types::MiiimgImageId,
    dst: &mut [u8],
) -> Result<(), LoadImageError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::LOAD_IMAGE)
        .with_data_value(&id)
        .add_output_buffer(OutputBuffer::new(dst, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(LoadImageError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(LoadImageError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`initialize`].
#[derive(Debug, thiserror::Error)]
pub enum InitializeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`reload`].
#[derive(Debug, thiserror::Error)]
pub enum ReloadError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`get_count`].
#[derive(Debug, thiserror::Error)]
pub enum GetCountError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`is_empty`].
#[derive(Debug, thiserror::Error)]
pub enum IsEmptyError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`is_full`].
#[derive(Debug, thiserror::Error)]
pub enum IsFullError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`get_attribute`].
#[derive(Debug, thiserror::Error)]
pub enum GetAttributeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`load_image`].
#[derive(Debug, thiserror::Error)]
pub enum LoadImageError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
