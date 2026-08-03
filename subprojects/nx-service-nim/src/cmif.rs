//! CMIF protocol operations for the nim service.

use nx_sf::{
    cmif,
    hipc::{BufferMode, OutputBuffer},
    service::BorrowedSessionHandle,
};

use crate::{proto, types::SystemUpdateTaskId};

/// Destroys a system update task.
pub fn destroy_system_update_task(
    session: BorrowedSessionHandle<'_>,
    task_id: &SystemUpdateTaskId,
) -> Result<(), DestroySystemUpdateTaskError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::DESTROY_SYSTEM_UPDATE_TASK)
        .with_data_value(task_id)
        .build();
    req.send(&mut buf, session)
        .map_err(DestroySystemUpdateTaskError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DestroySystemUpdateTaskError::ParseResponse)?;

    Ok(())
}

/// Lists all system update tasks.
///
/// Fills `out` with up to `out.len()` task IDs and returns the total count
/// reported by the service.
pub fn list_system_update_task(
    session: BorrowedSessionHandle<'_>,
    out: &mut [SystemUpdateTaskId],
) -> Result<i32, ListSystemUpdateTaskError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    // SAFETY: `out` is a valid `&mut` slice; viewing it as a byte slice
    // for the OUT buffer is sound, and the byte slice borrows `out`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(out.as_mut_ptr().cast::<u8>(), core::mem::size_of_val(out))
    };
    let req = cmif::CmifRequestBuilder::new(proto::LIST_SYSTEM_UPDATE_TASK)
        .add_output_buffer(OutputBuffer::new(out_bytes, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(ListSystemUpdateTaskError::SendRequest)?;

    let resp =
        cmif::parse_response::<&i32>(&buf).map_err(ListSystemUpdateTaskError::ParseResponse)?;

    let count = *resp.payload;

    Ok(count)
}

/// Error returned by [`destroy_system_update_task`].
#[derive(Debug, thiserror::Error)]
pub enum DestroySystemUpdateTaskError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`list_system_update_task`].
#[derive(Debug, thiserror::Error)]
pub enum ListSystemUpdateTaskError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
