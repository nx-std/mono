//! CMIF protocol operations for the BPC service.

use core::ptr;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::{proto, types::SleepButtonState};

/// Initiates a full system shutdown.
pub fn shutdown_system(session: SessionHandle) -> Result<(), ShutdownSystemError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SHUTDOWN_SYSTEM).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(ShutdownSystemError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(ShutdownSystemError::ParseResponse)?;

    Ok(())
}

/// Initiates a full system reboot.
pub fn reboot_system(session: SessionHandle) -> Result<(), RebootSystemError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::REBOOT_SYSTEM).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(RebootSystemError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(RebootSystemError::ParseResponse)?;

    Ok(())
}

/// Gets the current sleep button state.
///
/// Only available on HOS [2.0.0–13.2.1]. The caller must ensure the
/// correct HOS version before invoking this command.
pub fn get_sleep_button_state(
    session: SessionHandle,
) -> Result<SleepButtonState, GetSleepButtonStateError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_SLEEP_BUTTON_STATE).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetSleepButtonStateError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u8>()) }
        .map_err(GetSleepButtonStateError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u8.
    let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    SleepButtonState::from_raw(raw).ok_or(GetSleepButtonStateError::UnknownState(raw))
}

/// Gets whether the power button is currently pushed.
///
/// Only available on HOS [6.0.0+]. The caller must ensure the correct
/// HOS version before invoking this command.
pub fn get_power_button(session: SessionHandle) -> Result<bool, GetPowerButtonError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_POWER_BUTTON).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetPowerButtonError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u8>()) }
        .map_err(GetPowerButtonError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u8.
    let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(raw != 0)
}

/// Error returned by [`shutdown_system`].
#[derive(Debug, thiserror::Error)]
pub enum ShutdownSystemError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`reboot_system`].
#[derive(Debug, thiserror::Error)]
pub enum RebootSystemError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`get_sleep_button_state`].
#[derive(Debug, thiserror::Error)]
pub enum GetSleepButtonStateError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    #[error("unknown sleep button state: {0}")]
    UnknownState(u8),
}

/// Error returned by [`get_power_button`].
#[derive(Debug, thiserror::Error)]
pub enum GetPowerButtonError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
