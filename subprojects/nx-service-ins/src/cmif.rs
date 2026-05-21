//! CMIF protocol operations for the INS services.

use core::{mem::size_of, ptr};

use nx_sf::{
    cmif,
    ipc::{self, Handle as SessionHandle},
};

use crate::proto;

/// Gets the last system tick an event was signaled at.
pub fn get_last_tick(session: SessionHandle, id: u32) -> Result<u64, GetLastTickError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(proto::GET_LAST_TICK)
            .data_size(size_of::<u32>())
            .send(&mut buf)
            .map_err(GetLastTickError::BuildRequest)?;

        // SAFETY: `req` is exactly `size_of::<u32>()` bytes.
        unsafe {
            ptr::write_unaligned(req.as_mut_ptr().cast::<u32>(), id);
        }
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(GetLastTickError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, size_of::<u64>())
        .map_err(GetLastTickError::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<u64>()` bytes.
    let tick = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(tick)
}

/// Input layout for event commands: `{ u32 id, u64 unk }` with C alignment.
#[repr(C)]
struct EventInput {
    id: u32,
    _pad: u32,
    unk: u64,
}

/// Gets a readable event handle for the given request ID.
pub fn get_readable_event(
    session: SessionHandle,
    id: u32,
) -> Result<nx_svc::sync::EventHandle, GetReadableEventError> {
    let input = EventInput {
        id,
        _pad: 0,
        unk: 0,
    };

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(proto::GET_READABLE_EVENT)
            .data_size(size_of::<EventInput>())
            .send(&mut buf)
            .map_err(GetReadableEventError::BuildRequest)?;

        // SAFETY: `req` is exactly `size_of::<EventInput>()` bytes.
        unsafe {
            ptr::write_unaligned(req.as_mut_ptr().cast::<EventInput>(), input);
        }
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(GetReadableEventError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, 0).map_err(GetReadableEventError::ParseResponse)?;

    let handle = resp
        .copy_handles
        .first()
        .copied()
        .ok_or(GetReadableEventError::MissingHandle)?;

    // SAFETY: handle is from a valid IPC response.
    Ok(unsafe { nx_svc::sync::EventHandle::from_raw(handle) })
}

/// Gets a writable event handle for the given send ID.
pub fn get_writable_event(session: SessionHandle, id: u32) -> Result<u32, GetWritableEventError> {
    let input = EventInput {
        id,
        _pad: 0,
        unk: 0,
    };

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(proto::GET_WRITABLE_EVENT)
            .data_size(size_of::<EventInput>())
            .send(&mut buf)
            .map_err(GetWritableEventError::BuildRequest)?;

        // SAFETY: `req` is exactly `size_of::<EventInput>()` bytes.
        unsafe {
            ptr::write_unaligned(req.as_mut_ptr().cast::<EventInput>(), input);
        }
        ipc::send_sync_request(&mut buf, session).map_err(GetWritableEventError::SendRequest)?;
    }

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, 0).map_err(GetWritableEventError::ParseResponse)?;

    let handle = resp
        .copy_handles
        .first()
        .copied()
        .ok_or(GetWritableEventError::MissingHandle)?;

    Ok(handle)
}

/// Error returned by [`get_last_tick`].
#[derive(Debug, thiserror::Error)]
pub enum GetLastTickError {
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

/// Error returned by [`get_readable_event`].
#[derive(Debug, thiserror::Error)]
pub enum GetReadableEventError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// Response did not contain the expected copy handle.
    #[error("missing event handle in response")]
    MissingHandle,
}

/// Error returned by [`get_writable_event`].
#[derive(Debug, thiserror::Error)]
pub enum GetWritableEventError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// Response did not contain the expected copy handle.
    #[error("missing event handle in response")]
    MissingHandle,
}
