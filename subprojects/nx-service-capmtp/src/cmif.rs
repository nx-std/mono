//! CMIF protocol operations for the capture MTP service.

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Domain,
    DomainObject,
    OutHandleAttr,
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::{
        dispatch_no_io,
        dispatch_out,
    },
    proto,
    types::SessionOpenIn,
};

/// Opens a session sub-object on the root domain service.
pub(crate) fn open_session<'d>(domain: &'d Domain) -> Result<DomainObject<'d>, OpenSessionError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let mut result = domain
        .dispatch(proto::OPEN_SESSION)
        .out_objects(1)
        .send(&mut buf)
        .map_err(OpenSessionError::Dispatch)?;

    result.take_object(0).ok_or(OpenSessionError::MissingObject)
}

/// Opens the MTP session with transfer memory, folder/image/video limits,
/// and a UTF-16 device name.
#[allow(clippy::too_many_arguments)]
pub(crate) fn session_open(
    object: &DomainObject<'_>,
    tmem_handle: u32,
    tmem_size: u32,
    folder_count: u32,
    max_images: u32,
    max_videos: u32,
    name_utf16: &[u16],
) -> Result<(), SessionOpenError> {
    let input = SessionOpenIn {
        tmem_size,
        folder_count,
        max_images,
        max_videos,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::SESSION_OPEN)
        .in_raw(input.as_bytes())
        .in_buffer(name_utf16.as_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .in_handle(tmem_handle)
        .send(&mut buf)
        .map(|_| ())
        .map_err(SessionOpenError)
}

/// Closes the MTP session.
pub(crate) fn session_close(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::SESSION_CLOSE)
}

/// Starts the MTP command handler.
pub(crate) fn session_start_command_handler(
    object: &DomainObject<'_>,
) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::SESSION_START_COMMAND_HANDLER)
}

/// Stops the MTP command handler.
pub(crate) fn session_stop_command_handler(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::SESSION_STOP_COMMAND_HANDLER)
}

/// Checks whether the command handler is running.
pub(crate) fn session_is_running(object: &DomainObject<'_>) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(object, proto::SESSION_IS_RUNNING)?;
    Ok(val & 1 != 0)
}

/// Gets the connection event handle (copy handle).
pub(crate) fn session_get_connection_event(
    object: &DomainObject<'_>,
) -> Result<u32, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(proto::SESSION_GET_CONNECTION_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)?;

    Ok(result.copy_handles[0])
}

/// Checks whether a USB device is connected.
pub(crate) fn session_is_connected(object: &DomainObject<'_>) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(object, proto::SESSION_IS_CONNECTED)?;
    Ok(val & 1 != 0)
}

/// Gets the scan-error event handle (copy handle).
pub(crate) fn session_get_scan_error_event(
    object: &DomainObject<'_>,
) -> Result<u32, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(proto::SESSION_GET_SCAN_ERROR_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)?;

    Ok(result.copy_handles[0])
}

/// Gets the scan-error result code.
pub(crate) fn session_get_scan_error(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::SESSION_GET_SCAN_ERROR)
}

/// Error returned by [`open_session`].
#[derive(Debug, thiserror::Error)]
pub enum OpenSessionError {
    /// IPC dispatch failed.
    #[error("failed to dispatch OpenSession")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected sub-object id.
    #[error("OpenSession response did not include the expected sub-object")]
    MissingObject,
}

/// Error returned by [`session_open`].
#[derive(Debug, thiserror::Error)]
#[error("failed to open MTP session")]
pub struct SessionOpenError(#[source] pub DispatchError);
