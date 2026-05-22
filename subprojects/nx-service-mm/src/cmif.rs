//! CMIF protocol operations for the multimedia service.

use core::{mem::size_of, ptr};

use nx_sf::{
    cmif,
    ipc::{self, Handle as SessionHandle},
};

use crate::{proto, types::MmuModuleId};

/// Initialises a multimedia request (2.0.0+).
///
/// Returns the server-assigned request ID.
pub fn request_initialize(
    session: SessionHandle,
    module: MmuModuleId,
    unk: u32,
    autoclear: bool,
) -> Result<u32, RequestInitializeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::INITIALIZE)
        .data_size(size_of::<[u32; 3]>())
        .build();
    req.write_to(&mut buf)
        .map_err(RequestInitializeError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<[u32; 3]>()` bytes.
    unsafe {
        let data_ptr = buf.as_array_mut().as_mut_ptr().cast::<u32>();
        ptr::write_unaligned(data_ptr, module.as_raw());
        ptr::write_unaligned(data_ptr.add(1), unk);
        ptr::write_unaligned(data_ptr.add(2), autoclear as u32);
    }

    ipc::send_sync_request(&mut buf, session).map_err(RequestInitializeError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u32>())
        .map_err(RequestInitializeError::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<u32>()` bytes per the size
    // argument passed to `parse_response_bytes`.
    let id = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(id)
}

/// Error returned by [`request_initialize`].
#[derive(Debug, thiserror::Error)]
pub enum RequestInitializeError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Initialises a multimedia request (legacy, pre-2.0.0).
///
/// Returns the server-assigned request ID.
pub fn request_initialize_legacy(
    session: SessionHandle,
    module: MmuModuleId,
    unk: u32,
    autoclear: bool,
) -> Result<u32, RequestInitializeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::INITIALIZE_OLD)
        .data_size(size_of::<[u32; 3]>())
        .build();
    req.write_to(&mut buf)
        .map_err(RequestInitializeError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<[u32; 3]>()` bytes.
    unsafe {
        let data_ptr = buf.as_array_mut().as_mut_ptr().cast::<u32>();
        ptr::write_unaligned(data_ptr, module.as_raw());
        ptr::write_unaligned(data_ptr.add(1), unk);
        ptr::write_unaligned(data_ptr.add(2), autoclear as u32);
    }

    ipc::send_sync_request(&mut buf, session).map_err(RequestInitializeError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u32>())
        .map_err(RequestInitializeError::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<u32>()` bytes per the size
    // argument passed to `parse_response_bytes`.
    let id = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(id)
}

/// Finalises a multimedia request (2.0.0+). Keyed by request ID.
pub fn request_finalize(
    session: SessionHandle,
    request_id: u32,
) -> Result<(), RequestFinalizeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::FINALIZE)
        .data_size(size_of::<u32>())
        .build();
    req.write_to(&mut buf)
        .map_err(RequestFinalizeError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<u32>()` bytes.
    unsafe {
        ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<u32>(), request_id);
    }

    ipc::send_sync_request(&mut buf, session).map_err(RequestFinalizeError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(RequestFinalizeError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`request_finalize`] / [`request_finalize_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum RequestFinalizeError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Finalises a multimedia request (legacy, pre-2.0.0). Keyed by module ID.
pub fn request_finalize_legacy(
    session: SessionHandle,
    module: MmuModuleId,
) -> Result<(), RequestFinalizeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::FINALIZE_OLD)
        .data_size(size_of::<u32>())
        .build();
    req.write_to(&mut buf)
        .map_err(RequestFinalizeError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<u32>()` bytes.
    unsafe {
        ptr::write_unaligned(
            buf.as_array_mut().as_mut_ptr().cast::<u32>(),
            module.as_raw(),
        );
    }

    ipc::send_sync_request(&mut buf, session).map_err(RequestFinalizeError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(RequestFinalizeError::ParseResponse)?;

    Ok(())
}

/// Sets the frequency and waits (2.0.0+). Keyed by request ID.
pub fn request_set_and_wait(
    session: SessionHandle,
    request_id: u32,
    freq_hz: u32,
    timeout: i32,
) -> Result<(), RequestSetAndWaitError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::SET_AND_WAIT)
        .data_size(size_of::<[u32; 3]>())
        .build();
    req.write_to(&mut buf)
        .map_err(RequestSetAndWaitError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<[u32; 3]>()` bytes.
    unsafe {
        let data_ptr = buf.as_array_mut().as_mut_ptr().cast::<u32>();
        ptr::write_unaligned(data_ptr, request_id);
        ptr::write_unaligned(data_ptr.add(1), freq_hz);
        ptr::write_unaligned(data_ptr.add(2), timeout as u32);
    }

    ipc::send_sync_request(&mut buf, session).map_err(RequestSetAndWaitError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(RequestSetAndWaitError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`request_set_and_wait`] / [`request_set_and_wait_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum RequestSetAndWaitError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Sets the frequency and waits (legacy, pre-2.0.0). Keyed by module ID.
pub fn request_set_and_wait_legacy(
    session: SessionHandle,
    module: MmuModuleId,
    freq_hz: u32,
    timeout: i32,
) -> Result<(), RequestSetAndWaitError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::SET_AND_WAIT_OLD)
        .data_size(size_of::<[u32; 3]>())
        .build();
    req.write_to(&mut buf)
        .map_err(RequestSetAndWaitError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<[u32; 3]>()` bytes.
    unsafe {
        let data_ptr = buf.as_array_mut().as_mut_ptr().cast::<u32>();
        ptr::write_unaligned(data_ptr, module.as_raw());
        ptr::write_unaligned(data_ptr.add(1), freq_hz);
        ptr::write_unaligned(data_ptr.add(2), timeout as u32);
    }

    ipc::send_sync_request(&mut buf, session).map_err(RequestSetAndWaitError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(RequestSetAndWaitError::ParseResponse)?;

    Ok(())
}

/// Gets the current frequency in Hz (2.0.0+). Keyed by request ID.
pub fn request_get(session: SessionHandle, request_id: u32) -> Result<u32, RequestGetError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::GET)
        .data_size(size_of::<u32>())
        .build();
    req.write_to(&mut buf)
        .map_err(RequestGetError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<u32>()` bytes.
    unsafe {
        ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<u32>(), request_id);
    }

    ipc::send_sync_request(&mut buf, session).map_err(RequestGetError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u32>())
        .map_err(RequestGetError::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<u32>()` bytes per the size
    // argument passed to `parse_response_bytes`.
    let freq_hz = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(freq_hz)
}

/// Error returned by [`request_get`] / [`request_get_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum RequestGetError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Gets the current frequency in Hz (legacy, pre-2.0.0). Keyed by module ID.
pub fn request_get_legacy(
    session: SessionHandle,
    module: MmuModuleId,
) -> Result<u32, RequestGetError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::GET_OLD)
        .data_size(size_of::<u32>())
        .build();
    req.write_to(&mut buf)
        .map_err(RequestGetError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<u32>()` bytes.
    unsafe {
        ptr::write_unaligned(
            buf.as_array_mut().as_mut_ptr().cast::<u32>(),
            module.as_raw(),
        );
    }

    ipc::send_sync_request(&mut buf, session).map_err(RequestGetError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u32>())
        .map_err(RequestGetError::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<u32>()` bytes per the size
    // argument passed to `parse_response_bytes`.
    let freq_hz = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(freq_hz)
}
