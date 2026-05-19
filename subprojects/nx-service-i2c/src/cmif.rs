//! CMIF protocol operations for the I2C service.

use core::{mem::size_of, ptr};

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
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, proto::OPEN_SESSION)
            .data_size(size_of::<u32>())
            .send()
            .map_err(OpenSessionError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<u32>()` bytes.
        unsafe {
            ptr::write_unaligned(req.data.as_mut_ptr().cast::<u32>(), device);
        }
    }

    ipc::send_sync_request(session).map_err(OpenSessionError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, 0).map_err(OpenSessionError::ParseResponse)?;

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

    // SAFETY: `option_raw` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const option_raw).cast::<u8>(), size_of::<u32>())
    };
    service
        .dispatch(proto::SEND_AUTO)
        .in_raw(in_bytes)
        .in_buffer(buf, BufferAttr::HIPC_AUTO_SELECT)
        .send()
        .map(|_| ())
        .map_err(SendAutoError)
}

/// Receives data from an I2C device with automatic buffer selection.
pub fn receive_auto(
    service: &Session,
    buf: &mut [u8],
    option: I2cTransactionOption,
) -> Result<(), ReceiveAutoError> {
    let option_raw: u32 = option.bits();

    // SAFETY: `option_raw` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const option_raw).cast::<u8>(), size_of::<u32>())
    };
    service
        .dispatch(proto::RECEIVE_AUTO)
        .in_raw(in_bytes)
        .out_buffer(buf, BufferAttr::HIPC_AUTO_SELECT)
        .send()
        .map(|_| ())
        .map_err(ReceiveAutoError)
}

/// Executes a command list on the I2C device.
pub fn execute_command_list(
    service: &Session,
    dst: &mut [u8],
    cmd_list: &[u8],
) -> Result<(), ExecuteCommandListError> {
    service
        .dispatch(proto::EXECUTE_COMMAND_LIST)
        .out_buffer(dst, BufferAttr::HIPC_AUTO_SELECT)
        .in_buffer(cmd_list, BufferAttr::HIPC_POINTER)
        .send()
        .map(|_| ())
        .map_err(ExecuteCommandListError)
}

/// Error returned by [`open_session`].
#[derive(Debug, thiserror::Error)]
pub enum OpenSessionError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
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
