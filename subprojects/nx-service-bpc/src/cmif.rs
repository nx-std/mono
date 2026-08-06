//! CMIF protocol operations for the BPC service.

use nx_sf::{
    cmif,
    service::BorrowedSessionHandle,
};

use crate::{
    proto,
    types::SleepButtonState,
};

/// Initiates a full system shutdown.
pub fn shutdown_system(session: BorrowedSessionHandle<'_>) -> Result<(), ShutdownSystemError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::SHUTDOWN_SYSTEM).build();
    req.send(&mut buf, session)
        .map_err(ShutdownSystemError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(ShutdownSystemError::ParseResponse)?;

    Ok(())
}

/// Initiates a full system reboot.
pub fn reboot_system(session: BorrowedSessionHandle<'_>) -> Result<(), RebootSystemError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::REBOOT_SYSTEM).build();
    req.send(&mut buf, session)
        .map_err(RebootSystemError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(RebootSystemError::ParseResponse)?;

    Ok(())
}

/// Gets the current sleep button state.
///
/// Only available on HOS [2.0.0–13.2.1]. The caller must ensure the
/// correct HOS version before invoking this command.
pub fn get_sleep_button_state(
    session: BorrowedSessionHandle<'_>,
) -> Result<SleepButtonState, GetSleepButtonStateError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::GET_SLEEP_BUTTON_STATE).build();
    req.send(&mut buf, session)
        .map_err(GetSleepButtonStateError::SendRequest)?;

    let resp =
        cmif::parse_response::<&u8>(&buf).map_err(GetSleepButtonStateError::ParseResponse)?;

    let raw = *resp.payload;

    SleepButtonState::from_raw(raw).ok_or(GetSleepButtonStateError::UnknownState(raw))
}

/// Gets whether the power button is currently pushed.
///
/// Only available on HOS [6.0.0+]. The caller must ensure the correct
/// HOS version before invoking this command.
pub fn get_power_button(session: BorrowedSessionHandle<'_>) -> Result<bool, GetPowerButtonError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::GET_POWER_BUTTON).build();
    req.send(&mut buf, session)
        .map_err(GetPowerButtonError::SendRequest)?;

    let resp = cmif::parse_response::<&u8>(&buf).map_err(GetPowerButtonError::ParseResponse)?;

    let raw = *resp.payload;

    Ok(raw != 0)
}

/// Error returned by [`shutdown_system`].
#[derive(Debug, thiserror::Error)]
pub enum ShutdownSystemError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`reboot_system`].
#[derive(Debug, thiserror::Error)]
pub enum RebootSystemError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`get_sleep_button_state`].
#[derive(Debug, thiserror::Error)]
pub enum GetSleepButtonStateError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    #[error("unknown sleep button state: {0}")]
    UnknownState(u8),
}

/// Error returned by [`get_power_button`].
#[derive(Debug, thiserror::Error)]
pub enum GetPowerButtonError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
