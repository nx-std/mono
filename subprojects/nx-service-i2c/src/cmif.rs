//! CMIF protocol operations for the I2C service.

use core::ptr;

use nx_sf::{
    cmif,
    service::{BufferAttr, Session},
};
use nx_svc::ipc::{self, Handle};

use crate::{proto, types::I2cTransactionOption};

/// Opens an I2C session for the specified device.
///
/// Returns a [`Session`] representing the opened session.
pub fn open_session(session: Handle, device: u32) -> Result<Session, OpenSessionError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::OPEN_SESSION)
        .data_size(size_of::<u32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), device);
    }

    ipc::send_sync_request(session).map_err(OpenSessionError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(OpenSessionError::ParseResponse)?;

    let raw_handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(OpenSessionError::MissingHandle)?;

    // SAFETY: the kernel returned a valid move handle for the new device
    // session; ownership transfers to the new `Session`.
    let handle = unsafe { Handle::from_raw(raw_handle) };

    Ok(Session::from_handle(handle, 0))
}

/// Sends data to an I2C device with automatic buffer selection.
pub fn send_auto(
    service: &Session,
    buf: &[u8],
    option: I2cTransactionOption,
) -> Result<(), SendAutoError> {
    let option_raw: u32 = option.bits();

    // SAFETY: `option_raw` lives on the stack until `send()` returns.
    unsafe {
        service
            .dispatch(proto::SEND_AUTO)
            .in_raw((&raw const option_raw).cast::<u8>(), size_of::<u32>())
            .buffer(
                buf.as_ptr(),
                buf.len(),
                BufferAttr::IN.or(BufferAttr::HIPC_AUTO_SELECT),
            )
            .send()
            .map(|_| ())
            .map_err(SendAutoError)
    }
}

/// Receives data from an I2C device with automatic buffer selection.
pub fn receive_auto(
    service: &Session,
    buf: &mut [u8],
    option: I2cTransactionOption,
) -> Result<(), ReceiveAutoError> {
    let option_raw: u32 = option.bits();

    // SAFETY: `option_raw` lives on the stack until `send()` returns.
    unsafe {
        service
            .dispatch(proto::RECEIVE_AUTO)
            .in_raw((&raw const option_raw).cast::<u8>(), size_of::<u32>())
            .buffer(
                buf.as_mut_ptr(),
                buf.len(),
                BufferAttr::OUT.or(BufferAttr::HIPC_AUTO_SELECT),
            )
            .send()
            .map(|_| ())
            .map_err(ReceiveAutoError)
    }
}

/// Executes a command list on the I2C device.
pub fn execute_command_list(
    service: &Session,
    dst: &mut [u8],
    cmd_list: &[u8],
) -> Result<(), ExecuteCommandListError> {
    service
        .dispatch(proto::EXECUTE_COMMAND_LIST)
        .buffer(
            dst.as_mut_ptr(),
            dst.len(),
            BufferAttr::OUT.or(BufferAttr::HIPC_AUTO_SELECT),
        )
        .buffer(
            cmd_list.as_ptr(),
            cmd_list.len(),
            BufferAttr::IN.or(BufferAttr::HIPC_POINTER),
        )
        .send()
        .map(|_| ())
        .map_err(ExecuteCommandListError)
}

/// Error returned by [`open_session`].
#[derive(Debug, thiserror::Error)]
pub enum OpenSessionError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Response did not contain the expected move handle.
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by [`send_auto`].
#[derive(Debug, thiserror::Error)]
#[error("failed to send I2C data")]
pub struct SendAutoError(#[source] pub nx_sf::service::DispatchError);

/// Error returned by [`receive_auto`].
#[derive(Debug, thiserror::Error)]
#[error("failed to receive I2C data")]
pub struct ReceiveAutoError(#[source] pub nx_sf::service::DispatchError);

/// Error returned by [`execute_command_list`].
#[derive(Debug, thiserror::Error)]
#[error("failed to execute I2C command list")]
pub struct ExecuteCommandListError(#[source] pub nx_sf::service::DispatchError);
