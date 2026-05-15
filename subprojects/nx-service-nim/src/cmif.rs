//! CMIF protocol operations for the nim service.

use core::{mem::size_of, ptr};

use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::{proto, types::SystemUpdateTaskId};

/// Destroys a system update task.
pub fn destroy_system_update_task(
    session: SessionHandle,
    task_id: &SystemUpdateTaskId,
) -> Result<(), DestroySystemUpdateTaskError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, proto::DESTROY_SYSTEM_UPDATE_TASK)
            .data_size(size_of::<SystemUpdateTaskId>())
            .send()
            .map_err(DestroySystemUpdateTaskError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<SystemUpdateTaskId>()` bytes.
        unsafe {
            ptr::write_unaligned(req.data.as_mut_ptr().cast::<SystemUpdateTaskId>(), *task_id);
        }
    }

    ipc::send_sync_request(session).map_err(DestroySystemUpdateTaskError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(buf.as_array(), 0)
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
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        // SAFETY: `out` is a valid `&mut` slice; viewing it as a byte slice
        // for the OUT buffer is sound, and the byte slice borrows `out`.
        let out_bytes = unsafe {
            core::slice::from_raw_parts_mut(
                out.as_mut_ptr().cast::<u8>(),
                core::mem::size_of_val(out),
            )
        };
        cmif::CmifBuilder::new(&mut buf, proto::LIST_SYSTEM_UPDATE_TASK)
            .add_out_buffer(out_bytes.as_mut_ptr(), out_bytes.len(), BufferMode::Normal)
            .send()
            .map_err(ListSystemUpdateTaskError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(ListSystemUpdateTaskError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(buf.as_array(), size_of::<i32>())
        .map_err(ListSystemUpdateTaskError::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<i32>()` bytes.
    let count = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };

    Ok(count)
}

/// Error returned by [`destroy_system_update_task`].
#[derive(Debug, thiserror::Error)]
pub enum DestroySystemUpdateTaskError {
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

/// Error returned by [`list_system_update_task`].
#[derive(Debug, thiserror::Error)]
pub enum ListSystemUpdateTaskError {
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
