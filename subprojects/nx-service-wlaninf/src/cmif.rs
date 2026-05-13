//! CMIF protocol operations for the wlan:inf service.

use core::mem::size_of;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto::{CMD_GET_RSSI, CMD_GET_STATE, Rssi, WlanInfState};

/// Reads the current [`WlanInfState`] (cmd 10).
pub fn get_state(session: SessionHandle) -> Result<WlanInfState, GetStateError> {
    let raw = dispatch_no_in_u32(session, CMD_GET_STATE).map_err(GetStateError::Dispatch)?;
    WlanInfState::from_raw(raw).ok_or(GetStateError::InvalidState(raw))
}

/// Error returned by [`get_state`].
#[derive(Debug, thiserror::Error)]
pub enum GetStateError {
    /// CMIF dispatch failed.
    #[error("failed to dispatch GetState")]
    Dispatch(#[source] DispatchError),
    /// Service returned a state value outside the documented range.
    #[error("invalid WlanInfState: {0}")]
    InvalidState(u32),
}

/// Reads the current [`Rssi`] (cmd 12).
pub fn get_rssi(session: SessionHandle) -> Result<Rssi, GetRssiError> {
    let raw = dispatch_no_in_u32(session, CMD_GET_RSSI).map_err(GetRssiError)?;
    // libnx reinterprets the same 4-byte payload through an `s32*` cast; the
    // service does not zero/extend, so a bitwise reinterpret preserves the
    // wire value.
    Ok(Rssi::from_raw(raw as i32))
}

/// Error returned by [`get_rssi`]: CMIF dispatch failure.
#[derive(Debug, thiserror::Error)]
#[error("failed to dispatch GetRSSI")]
pub struct GetRssiError(#[source] pub DispatchError);

/// Sends a CMIF request with no input payload and reads a single `u32` from
/// the response data area.
fn dispatch_no_in_u32(session: SessionHandle, cmd_id: u32) -> Result<u32, DispatchError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id).build();
    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u32>()) }
        .map_err(DispatchError::ParseResponse)?;

    if resp.data.len() < size_of::<u32>() {
        return Err(DispatchError::ShortResponse);
    }
    Ok(u32::from_le_bytes([
        resp.data[0],
        resp.data[1],
        resp.data[2],
        resp.data[3],
    ]))
}

/// Low-level error returned by [`dispatch_no_in_u32`].
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Response payload was shorter than the expected 4 bytes.
    #[error("response payload shorter than 4 bytes")]
    ShortResponse,
}
