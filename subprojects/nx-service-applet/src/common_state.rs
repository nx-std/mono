//! ICommonStateGetter interface operations.
//!
//! This module implements commands for the ICommonStateGetter sub-interface,
//! which provides access to applet state information like focus, operation mode,
//! and message events.

use core::{mem::size_of, ptr};

use nx_sf::service::{DispatchError, OutHandleAttr, Service};
use nx_svc::sync::EventHandle;

use crate::proto::{
    AppletFocusState, AppletMessage, AppletOperationMode, CMD_CSG_GET_CURRENT_FOCUS_STATE,
    CMD_CSG_GET_EVENT_HANDLE, CMD_CSG_GET_OPERATION_MODE, CMD_CSG_GET_PERFORMANCE_MODE,
    CMD_CSG_RECEIVE_MESSAGE,
};

/// Gets the message event handle from ICommonStateGetter.
///
/// This handle is signaled when the applet receives a message.
pub fn get_event_handle(csg: &Service) -> Result<EventHandle, GetEventHandleError> {
    let result = csg
        .dispatch(CMD_CSG_GET_EVENT_HANDLE)
        .out_handle(0, OutHandleAttr::Copy)
        .send()
        .map_err(GetEventHandleError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(GetEventHandleError::MissingHandle);
    }

    // SAFETY: Kernel returned a valid event handle in the response.
    Ok(unsafe { EventHandle::from_raw(result.copy_handles[0]) })
}

/// Error returned by [`get_event_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetEventHandleError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response did not contain the expected handle.
    #[error("missing handle in response")]
    MissingHandle,
}

/// Receives a pending message from ICommonStateGetter.
///
/// Returns `Ok(None)` if no message is pending (error 0x680).
pub fn receive_message(csg: &Service) -> Result<Option<AppletMessage>, ReceiveMessageError> {
    let result = csg
        .dispatch(CMD_CSG_RECEIVE_MESSAGE)
        .out_size(size_of::<u32>())
        .send();

    match result {
        Ok(resp) => {
            if resp.data.len() < size_of::<u32>() {
                return Err(ReceiveMessageError::InvalidResponse);
            }

            // SAFETY: Response data contains u32 message type.
            let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

            Ok(AppletMessage::from_raw(raw))
        }
        Err(DispatchError::ParseResponse(err)) => {
            // Check for "no message available" error (0x680)
            if let nx_sf::cmif::ParseResponseError::ServiceError(code) = err
                && code == 0x680
            {
                return Ok(None);
            }
            Err(ReceiveMessageError::Dispatch(DispatchError::ParseResponse(
                err,
            )))
        }
        Err(err) => Err(ReceiveMessageError::Dispatch(err)),
    }
}

/// Error returned by [`receive_message`].
#[derive(Debug, thiserror::Error)]
pub enum ReceiveMessageError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response data was invalid.
    #[error("invalid response data")]
    InvalidResponse,
}

/// Gets the current operation mode from ICommonStateGetter.
pub fn get_operation_mode(csg: &Service) -> Result<AppletOperationMode, GetOperationModeError> {
    let result = csg
        .dispatch(CMD_CSG_GET_OPERATION_MODE)
        .out_size(size_of::<u8>())
        .send()
        .map_err(GetOperationModeError::Dispatch)?;

    if result.data.is_empty() {
        return Err(GetOperationModeError::InvalidResponse);
    }

    let raw = result.data[0];
    AppletOperationMode::from_raw(raw).ok_or(GetOperationModeError::InvalidValue(raw))
}

/// Error returned by [`get_operation_mode`].
#[derive(Debug, thiserror::Error)]
pub enum GetOperationModeError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response data was invalid.
    #[error("invalid response data")]
    InvalidResponse,
    /// Operation mode value was unknown.
    #[error("unknown operation mode value: {0}")]
    InvalidValue(u8),
}

/// Gets the current performance mode from ICommonStateGetter.
pub fn get_performance_mode(csg: &Service) -> Result<u32, GetPerformanceModeError> {
    let result = csg
        .dispatch(CMD_CSG_GET_PERFORMANCE_MODE)
        .out_size(size_of::<u32>())
        .send()
        .map_err(GetPerformanceModeError::Dispatch)?;

    if result.data.len() < size_of::<u32>() {
        return Err(GetPerformanceModeError::InvalidResponse);
    }

    // SAFETY: Response data contains u32 performance mode.
    let mode = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) };
    Ok(mode)
}

/// Error returned by [`get_performance_mode`].
#[derive(Debug, thiserror::Error)]
pub enum GetPerformanceModeError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response data was invalid.
    #[error("invalid response data")]
    InvalidResponse,
}

/// Gets the current focus state from ICommonStateGetter.
pub fn get_current_focus_state(
    csg: &Service,
) -> Result<AppletFocusState, GetCurrentFocusStateError> {
    let result = csg
        .dispatch(CMD_CSG_GET_CURRENT_FOCUS_STATE)
        .out_size(size_of::<u8>())
        .send()
        .map_err(GetCurrentFocusStateError::Dispatch)?;

    if result.data.is_empty() {
        return Err(GetCurrentFocusStateError::InvalidResponse);
    }

    let raw = result.data[0];
    AppletFocusState::from_raw(raw).ok_or(GetCurrentFocusStateError::InvalidValue(raw))
}

/// Error returned by [`get_current_focus_state`].
#[derive(Debug, thiserror::Error)]
pub enum GetCurrentFocusStateError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response data was invalid.
    #[error("invalid response data")]
    InvalidResponse,
    /// Focus state value was unknown.
    #[error("unknown focus state value: {0}")]
    InvalidValue(u8),
}
