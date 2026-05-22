//! Button event handle acquisition and activation commands.
//!
//! All commands in this group send PID + AppletResourceUserId.

use core::mem::size_of;

use nx_sf::service::{DispatchError, OutHandleAttr, Session};

use super::AcquireEventError;
use crate::proto;

/// Dispatches a PID+ARUID command that returns a copy handle.
fn dispatch_pid_aruid_out_handle(
    service: &Session,
    cmd_id: u32,
    aruid: u64,
) -> Result<u32, AcquireEventError> {
    // SAFETY: `aruid` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const aruid).cast::<u8>(), size_of::<u64>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send_pid()
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AcquireEventError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(AcquireEventError::MissingHandle);
    }

    Ok(result.copy_handles[0])
}

/// Dispatches a PID+ARUID command with no output.
fn dispatch_pid_aruid_no_out(
    service: &Session,
    cmd_id: u32,
    aruid: u64,
) -> Result<(), DispatchError> {
    // SAFETY: `aruid` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const aruid).cast::<u8>(), size_of::<u64>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send_pid()
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// AcquireHomeButtonEventHandle (cmd 101).
pub(crate) fn acquire_home_button_event_handle(
    service: &Session,
    aruid: u64,
) -> Result<u32, AcquireEventError> {
    dispatch_pid_aruid_out_handle(service, proto::ACQUIRE_HOME_BUTTON_EVENT_HANDLE, aruid)
}

/// ActivateHomeButton (cmd 111).
pub(crate) fn activate_home_button(service: &Session, aruid: u64) -> Result<(), DispatchError> {
    dispatch_pid_aruid_no_out(service, proto::ACTIVATE_HOME_BUTTON, aruid)
}

/// AcquireSleepButtonEventHandle (cmd 121).
pub(crate) fn acquire_sleep_button_event_handle(
    service: &Session,
    aruid: u64,
) -> Result<u32, AcquireEventError> {
    dispatch_pid_aruid_out_handle(service, proto::ACQUIRE_SLEEP_BUTTON_EVENT_HANDLE, aruid)
}

/// ActivateSleepButton (cmd 131).
pub(crate) fn activate_sleep_button(service: &Session, aruid: u64) -> Result<(), DispatchError> {
    dispatch_pid_aruid_no_out(service, proto::ACTIVATE_SLEEP_BUTTON, aruid)
}

/// AcquireCaptureButtonEventHandle (cmd 141).
pub(crate) fn acquire_capture_button_event_handle(
    service: &Session,
    aruid: u64,
) -> Result<u32, AcquireEventError> {
    dispatch_pid_aruid_out_handle(service, proto::ACQUIRE_CAPTURE_BUTTON_EVENT_HANDLE, aruid)
}

/// ActivateCaptureButton (cmd 151).
pub(crate) fn activate_capture_button(service: &Session, aruid: u64) -> Result<(), DispatchError> {
    dispatch_pid_aruid_no_out(service, proto::ACTIVATE_CAPTURE_BUTTON, aruid)
}
