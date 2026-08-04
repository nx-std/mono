//! Sub-object CMIF commands.
//!
//! - IProgressMonitorForDeleteUserSaveDataAll
//! - IProgressAsyncResult

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    OutHandleAttr,
    Session,
};

use crate::{
    dispatch::{
        dispatch_no_io,
        dispatch_out,
    },
    proto,
    types::ProgressForDeleteUserSaveDataAll,
};

// ---------------------------------------------------------------------------
// IProgressMonitorForDeleteUserSaveDataAll
// ---------------------------------------------------------------------------

/// GetSystemEvent (cmd 0) — returns copy handle.
pub(crate) fn progress_monitor_get_system_event(
    service: &Session,
) -> Result<u32, AcquireEventError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::PROGRESS_MONITOR_GET_SYSTEM_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AcquireEventError::Dispatch)?;

    let Some(handle) = result.copy_handles.first().copied() else {
        return Err(AcquireEventError::MissingHandle);
    };

    Ok(handle)
}

/// IsFinished (cmd 1).
#[inline]
pub(crate) fn progress_monitor_is_finished(service: &Session) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(service, proto::PROGRESS_MONITOR_IS_FINISHED)?;
    Ok(val & 1 != 0)
}

/// GetResult (cmd 2).
#[inline]
pub(crate) fn progress_monitor_get_result(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::PROGRESS_MONITOR_GET_RESULT)
}

/// GetProgress (cmd 10).
#[inline]
pub(crate) fn progress_monitor_get_progress(
    service: &Session,
) -> Result<ProgressForDeleteUserSaveDataAll, DispatchError> {
    dispatch_out(service, proto::PROGRESS_MONITOR_GET_PROGRESS)
}

// ---------------------------------------------------------------------------
// IProgressAsyncResult
// ---------------------------------------------------------------------------

/// Get (cmd 0).
#[inline]
pub(crate) fn progress_async_get(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::PROGRESS_ASYNC_GET)
}

/// Cancel (cmd 1).
#[inline]
pub(crate) fn progress_async_cancel(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::PROGRESS_ASYNC_CANCEL)
}

/// GetProgress (cmd 2).
pub(crate) fn progress_async_get_progress(
    service: &Session,
    out: &mut [u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::PROGRESS_ASYNC_GET_PROGRESS)
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// GetDetailResult (cmd 3).
#[inline]
pub(crate) fn progress_async_get_detail_result(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::PROGRESS_ASYNC_GET_DETAIL_RESULT)
}

/// GetErrorContext (cmd 4).
pub(crate) fn progress_async_get_error_context(
    service: &Session,
    out: &mut [u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::PROGRESS_ASYNC_GET_ERROR_CONTEXT)
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned by event acquisition commands.
#[derive(Debug, thiserror::Error)]
pub enum AcquireEventError {
    #[error("failed to dispatch event acquisition")]
    Dispatch(#[source] DispatchError),
    #[error("event acquisition response did not include expected copy handle")]
    MissingHandle,
}
