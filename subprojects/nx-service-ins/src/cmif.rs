//! CMIF protocol operations for the INS services.

use core::ptr;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto;

/// Gets the last system tick an event was signaled at.
pub fn get_last_tick(session: SessionHandle, id: u32) -> Result<u64, GetLastTickError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_LAST_TICK)
        .data_size(size_of::<u32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), id);
    }

    ipc::send_sync_request(session).map_err(GetLastTickError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u64>()) }
        .map_err(GetLastTickError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u64.
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_READABLE_EVENT)
        .data_size(size_of::<EventInput>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    let input = EventInput {
        id,
        _pad: 0,
        unk: 0,
    };

    // SAFETY: req.data points to valid payload area with space for EventInput.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<EventInput>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(GetReadableEventError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetReadableEventError::ParseResponse)?;

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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_WRITABLE_EVENT)
        .data_size(size_of::<EventInput>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    let input = EventInput {
        id,
        _pad: 0,
        unk: 0,
    };

    // SAFETY: req.data points to valid payload area with space for EventInput.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<EventInput>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(GetWritableEventError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetWritableEventError::ParseResponse)?;

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
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`get_readable_event`].
#[derive(Debug, thiserror::Error)]
pub enum GetReadableEventError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Response did not contain the expected copy handle.
    #[error("missing event handle in response")]
    MissingHandle,
}

/// Error returned by [`get_writable_event`].
#[derive(Debug, thiserror::Error)]
pub enum GetWritableEventError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Response did not contain the expected copy handle.
    #[error("missing event handle in response")]
    MissingHandle,
}
