//! CMIF protocol operations for the nim service.

use core::ptr;

use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::{proto, types::SystemUpdateTaskId};

/// Destroys a system update task.
pub fn destroy_system_update_task(
    session: SessionHandle,
    task_id: &SystemUpdateTaskId,
) -> Result<(), DestroySystemUpdateTaskError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::DESTROY_SYSTEM_UPDATE_TASK)
        .data_size(size_of::<SystemUpdateTaskId>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for SystemUpdateTaskId.
    unsafe {
        ptr::write_unaligned(
            req.data.as_ptr().cast::<SystemUpdateTaskId>().cast_mut(),
            *task_id,
        );
    }

    ipc::send_sync_request(session).map_err(DestroySystemUpdateTaskError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(DestroySystemUpdateTaskError::ParseResponse)?;

    Ok(())
}

/// Lists all system update tasks.
///
/// Fills `out` with up to `out.len()` task IDs and returns the total count
/// reported by the service.
pub fn list_system_update_task(
    session: SessionHandle,
    out: &mut [SystemUpdateTaskId],
) -> Result<i32, ListSystemUpdateTaskError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::LIST_SYSTEM_UPDATE_TASK)
        .out_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    req.add_out_buffer(
        out.as_mut_ptr().cast::<u8>(),
        size_of_val(out),
        BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(ListSystemUpdateTaskError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<i32>()) }
        .map_err(ListSystemUpdateTaskError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for i32.
    let count = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };

    Ok(count)
}

/// Error returned by [`destroy_system_update_task`].
#[derive(Debug, thiserror::Error)]
pub enum DestroySystemUpdateTaskError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`list_system_update_task`].
#[derive(Debug, thiserror::Error)]
pub enum ListSystemUpdateTaskError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
