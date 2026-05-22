//! CMIF protocol operations for the mii image service.

use core::{mem::size_of, ptr};

use nx_sf::{
    cmif,
    hipc::BufferMode,
    ipc::{self, Handle as SessionHandle},
};

use crate::{proto, types::MiiimgImageAttribute};

/// Initializes the image database.
///
/// Sends a `u8` mode value and returns a `u8` result.
pub fn initialize(session: SessionHandle, mode: u8) -> Result<u8, InitializeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::INITIALIZE)
        .data_value(&mode)
        .build();
    req.write_to(&mut buf)
        .map_err(InitializeError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(InitializeError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u8>())
        .map_err(InitializeError::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<u8>()` bytes per the size
    // argument passed to `parse_response_bytes`.
    let out = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(out)
}

/// Reloads the image database.
pub fn reload(session: SessionHandle) -> Result<(), ReloadError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::RELOAD).build();
    req.write_to(&mut buf).map_err(ReloadError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(ReloadError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(ReloadError::ParseResponse)?;

    Ok(())
}

/// Gets the number of mii images in the database.
pub fn get_count(session: SessionHandle) -> Result<i32, GetCountError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_COUNT).build();
    req.write_to(&mut buf)
        .map_err(GetCountError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(GetCountError::SendRequest)?;

    let resp =
        cmif::parse_response_bytes(&buf, size_of::<i32>()).map_err(GetCountError::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<i32>()` bytes per the size
    // argument passed to `parse_response_bytes`.
    let count = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };

    Ok(count)
}

/// Gets whether the image database is empty.
pub fn is_empty(session: SessionHandle) -> Result<bool, IsEmptyError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::IS_EMPTY).build();
    req.write_to(&mut buf).map_err(IsEmptyError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(IsEmptyError::SendRequest)?;

    let resp =
        cmif::parse_response_bytes(&buf, size_of::<u8>()).map_err(IsEmptyError::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<u8>()` bytes per the size
    // argument passed to `parse_response_bytes`.
    let val = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(val & 1 != 0)
}

/// Gets whether the image database is full.
pub fn is_full(session: SessionHandle) -> Result<bool, IsFullError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::IS_FULL).build();
    req.write_to(&mut buf).map_err(IsFullError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(IsFullError::SendRequest)?;

    let resp =
        cmif::parse_response_bytes(&buf, size_of::<u8>()).map_err(IsFullError::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<u8>()` bytes per the size
    // argument passed to `parse_response_bytes`.
    let val = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(val & 1 != 0)
}

/// Gets the image attribute for the specified index.
pub fn get_attribute(
    session: SessionHandle,
    index: i32,
) -> Result<MiiimgImageAttribute, GetAttributeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_ATTRIBUTE)
        .data_value(&index)
        .build();
    req.write_to(&mut buf)
        .map_err(GetAttributeError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(GetAttributeError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<MiiimgImageAttribute>())
        .map_err(GetAttributeError::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<MiiimgImageAttribute>()` bytes
    // per the size argument passed to `parse_response_bytes`.
    let attr = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<MiiimgImageAttribute>()) };

    Ok(attr)
}

/// Loads the image data (raw RGBA8) for the specified image ID.
pub fn load_image(
    session: SessionHandle,
    id: crate::types::MiiimgImageId,
    dst: &mut [u8],
) -> Result<(), LoadImageError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::LOAD_IMAGE)
        .data_value(&id)
        .add_out_buffer(dst.as_mut_ptr(), dst.len(), BufferMode::Normal)
        .build();
    req.write_to(&mut buf)
        .map_err(LoadImageError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(LoadImageError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(LoadImageError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`initialize`].
#[derive(Debug, thiserror::Error)]
pub enum InitializeError {
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

/// Error returned by [`reload`].
#[derive(Debug, thiserror::Error)]
pub enum ReloadError {
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

/// Error returned by [`get_count`].
#[derive(Debug, thiserror::Error)]
pub enum GetCountError {
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

/// Error returned by [`is_empty`].
#[derive(Debug, thiserror::Error)]
pub enum IsEmptyError {
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

/// Error returned by [`is_full`].
#[derive(Debug, thiserror::Error)]
pub enum IsFullError {
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

/// Error returned by [`get_attribute`].
#[derive(Debug, thiserror::Error)]
pub enum GetAttributeError {
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

/// Error returned by [`load_image`].
#[derive(Debug, thiserror::Error)]
pub enum LoadImageError {
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
