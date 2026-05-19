//! CMIF protocol operations for the wlan:inf service.

use core::{mem::size_of, ptr};

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
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifBuilder::new(&mut buf, cmd_id)
            .send()
            .map_err(DispatchError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp =
        cmif::parse_response_bytes(&buf, size_of::<u32>()).map_err(DispatchError::ParseResponse)?;

    // SAFETY: resp.data is at least size_of::<u32>() bytes as requested above.
    Ok(unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) })
}

/// Low-level error returned by [`dispatch_no_in_u32`].
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
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
