//! CMIF protocol operations for the wlan:inf service.

use nx_sf::{cmif, ipc::Handle as SessionHandle};

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
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmd_id).build();
    req.send(&mut buf, session)
        .map_err(DispatchError::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(DispatchError::ParseResponse)?;

    Ok(*resp.payload)
}

/// Low-level error returned by [`dispatch_no_in_u32`].
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
