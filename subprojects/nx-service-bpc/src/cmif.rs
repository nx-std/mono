//! CMIF protocol operations for the BPC service.

use core::{mem::size_of, ptr};

use nx_sf::{
    cmif,
    ipc::{self, Handle as SessionHandle},
};

use crate::{proto, types::SleepButtonState};

/// Initiates a full system shutdown.
pub fn shutdown_system(session: SessionHandle) -> Result<(), ShutdownSystemError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::SHUTDOWN_SYSTEM).build();
    req.write_to(&mut buf)
        .map_err(ShutdownSystemError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(ShutdownSystemError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(ShutdownSystemError::ParseResponse)?;

    Ok(())
}

/// Initiates a full system reboot.
pub fn reboot_system(session: SessionHandle) -> Result<(), RebootSystemError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::REBOOT_SYSTEM).build();
    req.write_to(&mut buf)
        .map_err(RebootSystemError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(RebootSystemError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(RebootSystemError::ParseResponse)?;

    Ok(())
}

/// Gets the current sleep button state.
///
/// Only available on HOS [2.0.0–13.2.1]. The caller must ensure the
/// correct HOS version before invoking this command.
pub fn get_sleep_button_state(
    session: SessionHandle,
) -> Result<SleepButtonState, GetSleepButtonStateError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_SLEEP_BUTTON_STATE).build();
    req.write_to(&mut buf)
        .map_err(GetSleepButtonStateError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(GetSleepButtonStateError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u8>())
        .map_err(GetSleepButtonStateError::ParseResponse)?;

    // SAFETY: parse_response_bytes guarantees at least size_of::<u8>() bytes in resp.data.
    let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    SleepButtonState::from_raw(raw).ok_or(GetSleepButtonStateError::UnknownState(raw))
}

/// Gets whether the power button is currently pushed.
///
/// Only available on HOS [6.0.0+]. The caller must ensure the correct
/// HOS version before invoking this command.
pub fn get_power_button(session: SessionHandle) -> Result<bool, GetPowerButtonError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_POWER_BUTTON).build();
    req.write_to(&mut buf)
        .map_err(GetPowerButtonError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(GetPowerButtonError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u8>())
        .map_err(GetPowerButtonError::ParseResponse)?;

    // SAFETY: parse_response_bytes guarantees at least size_of::<u8>() bytes in resp.data.
    let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(raw != 0)
}

/// Error returned by [`shutdown_system`].
#[derive(Debug, thiserror::Error)]
pub enum ShutdownSystemError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Error returned by [`reboot_system`].
#[derive(Debug, thiserror::Error)]
pub enum RebootSystemError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Error returned by [`get_sleep_button_state`].
#[derive(Debug, thiserror::Error)]
pub enum GetSleepButtonStateError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    #[error("unknown sleep button state: {0}")]
    UnknownState(u8),
}

/// Error returned by [`get_power_button`].
#[derive(Debug, thiserror::Error)]
pub enum GetPowerButtonError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}
