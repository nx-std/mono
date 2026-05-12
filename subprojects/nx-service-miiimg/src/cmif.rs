//! CMIF protocol operations for the mii image service.

use core::ptr;

use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::{proto, types::MiiimgImageAttribute};

/// Initializes the image database.
///
/// Sends a `u8` mode value and returns a `u8` result.
pub fn initialize(session: SessionHandle, mode: u8) -> Result<u8, InitializeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::INITIALIZE)
        .data_size(size_of::<u8>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u8.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u8>().cast_mut(), mode);
    }

    ipc::send_sync_request(session).map_err(InitializeError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u8>()) }
        .map_err(InitializeError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u8.
    let out = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(out)
}

/// Reloads the image database.
pub fn reload(session: SessionHandle) -> Result<(), ReloadError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::RELOAD).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(ReloadError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(ReloadError::ParseResponse)?;

    Ok(())
}

/// Gets the number of mii images in the database.
pub fn get_count(session: SessionHandle) -> Result<i32, GetCountError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_COUNT).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetCountError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<i32>()) }
        .map_err(GetCountError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for i32.
    let count = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };

    Ok(count)
}

/// Gets whether the image database is empty.
pub fn is_empty(session: SessionHandle) -> Result<bool, IsEmptyError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::IS_EMPTY).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(IsEmptyError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u8>()) }
        .map_err(IsEmptyError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u8.
    let val = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(val & 1 != 0)
}

/// Gets whether the image database is full.
pub fn is_full(session: SessionHandle) -> Result<bool, IsFullError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::IS_FULL).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(IsFullError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u8>()) }
        .map_err(IsFullError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u8.
    let val = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(val & 1 != 0)
}

/// Gets the image attribute for the specified index.
pub fn get_attribute(
    session: SessionHandle,
    index: i32,
) -> Result<MiiimgImageAttribute, GetAttributeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_ATTRIBUTE)
        .data_size(size_of::<i32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for i32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<i32>().cast_mut(), index);
    }

    ipc::send_sync_request(session).map_err(GetAttributeError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<MiiimgImageAttribute>()) }
        .map_err(GetAttributeError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for MiiimgImageAttribute.
    let attr = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<MiiimgImageAttribute>()) };

    Ok(attr)
}

/// Loads the image data (raw RGBA8) for the specified image ID.
pub fn load_image(
    session: SessionHandle,
    id: crate::types::MiiimgImageId,
    dst: &mut [u8],
) -> Result<(), LoadImageError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::LOAD_IMAGE)
        .data_size(size_of::<crate::types::MiiimgImageId>())
        .out_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for MiiimgImageId.
    unsafe {
        ptr::write_unaligned(
            req.data
                .as_ptr()
                .cast::<crate::types::MiiimgImageId>()
                .cast_mut(),
            id,
        );
    }

    req.add_out_buffer(dst.as_mut_ptr(), dst.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(LoadImageError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(LoadImageError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`initialize`].
#[derive(Debug, thiserror::Error)]
pub enum InitializeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`reload`].
#[derive(Debug, thiserror::Error)]
pub enum ReloadError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`get_count`].
#[derive(Debug, thiserror::Error)]
pub enum GetCountError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`is_empty`].
#[derive(Debug, thiserror::Error)]
pub enum IsEmptyError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`is_full`].
#[derive(Debug, thiserror::Error)]
pub enum IsFullError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`get_attribute`].
#[derive(Debug, thiserror::Error)]
pub enum GetAttributeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`load_image`].
#[derive(Debug, thiserror::Error)]
pub enum LoadImageError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
