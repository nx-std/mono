//! CMIF protocol operations for the multimedia service.

use core::ptr;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::{proto, types::MmuModuleId};

// ---------------------------------------------------------------------------
// request_initialize (2.0.0+)
// ---------------------------------------------------------------------------

/// Initialises a multimedia request (2.0.0+).
///
/// Returns the server-assigned request ID.
pub fn request_initialize(
    session: SessionHandle,
    module: MmuModuleId,
    unk: u32,
    autoclear: bool,
) -> Result<u32, RequestInitializeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::INITIALIZE)
        .data_size(size_of::<[u32; 3]>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for [u32; 3].
    unsafe {
        let data_ptr = req.data.as_ptr().cast::<u32>().cast_mut();
        ptr::write_unaligned(data_ptr, module.as_raw());
        ptr::write_unaligned(data_ptr.add(1), unk);
        ptr::write_unaligned(data_ptr.add(2), autoclear as u32);
    }

    ipc::send_sync_request(session).map_err(RequestInitializeError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u32>()) }
        .map_err(RequestInitializeError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u32.
    let id = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(id)
}

/// Error returned by [`request_initialize`].
#[derive(Debug, thiserror::Error)]
pub enum RequestInitializeError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

// ---------------------------------------------------------------------------
// request_initialize_legacy (pre-2.0.0)
// ---------------------------------------------------------------------------

/// Initialises a multimedia request (legacy, pre-2.0.0).
///
/// Returns the server-assigned request ID.
pub fn request_initialize_legacy(
    session: SessionHandle,
    module: MmuModuleId,
    unk: u32,
    autoclear: bool,
) -> Result<u32, RequestInitializeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::INITIALIZE_OLD)
        .data_size(size_of::<[u32; 3]>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for [u32; 3].
    unsafe {
        let data_ptr = req.data.as_ptr().cast::<u32>().cast_mut();
        ptr::write_unaligned(data_ptr, module.as_raw());
        ptr::write_unaligned(data_ptr.add(1), unk);
        ptr::write_unaligned(data_ptr.add(2), autoclear as u32);
    }

    ipc::send_sync_request(session).map_err(RequestInitializeError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u32>()) }
        .map_err(RequestInitializeError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u32.
    let id = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(id)
}

// ---------------------------------------------------------------------------
// request_finalize (2.0.0+)
// ---------------------------------------------------------------------------

/// Finalises a multimedia request (2.0.0+). Keyed by request ID.
pub fn request_finalize(
    session: SessionHandle,
    request_id: u32,
) -> Result<(), RequestFinalizeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::FINALIZE)
        .data_size(size_of::<u32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), request_id);
    }

    ipc::send_sync_request(session).map_err(RequestFinalizeError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(RequestFinalizeError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`request_finalize`] / [`request_finalize_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum RequestFinalizeError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

// ---------------------------------------------------------------------------
// request_finalize_legacy (pre-2.0.0)
// ---------------------------------------------------------------------------

/// Finalises a multimedia request (legacy, pre-2.0.0). Keyed by module ID.
pub fn request_finalize_legacy(
    session: SessionHandle,
    module: MmuModuleId,
) -> Result<(), RequestFinalizeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::FINALIZE_OLD)
        .data_size(size_of::<u32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), module.as_raw());
    }

    ipc::send_sync_request(session).map_err(RequestFinalizeError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(RequestFinalizeError::ParseResponse)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// request_set_and_wait (2.0.0+)
// ---------------------------------------------------------------------------

/// Sets the frequency and waits (2.0.0+). Keyed by request ID.
pub fn request_set_and_wait(
    session: SessionHandle,
    request_id: u32,
    freq_hz: u32,
    timeout: i32,
) -> Result<(), RequestSetAndWaitError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SET_AND_WAIT)
        .data_size(size_of::<[u32; 3]>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for [u32; 3].
    unsafe {
        let data_ptr = req.data.as_ptr().cast::<u32>().cast_mut();
        ptr::write_unaligned(data_ptr, request_id);
        ptr::write_unaligned(data_ptr.add(1), freq_hz);
        ptr::write_unaligned(data_ptr.add(2), timeout as u32);
    }

    ipc::send_sync_request(session).map_err(RequestSetAndWaitError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(RequestSetAndWaitError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`request_set_and_wait`] / [`request_set_and_wait_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum RequestSetAndWaitError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

// ---------------------------------------------------------------------------
// request_set_and_wait_legacy (pre-2.0.0)
// ---------------------------------------------------------------------------

/// Sets the frequency and waits (legacy, pre-2.0.0). Keyed by module ID.
pub fn request_set_and_wait_legacy(
    session: SessionHandle,
    module: MmuModuleId,
    freq_hz: u32,
    timeout: i32,
) -> Result<(), RequestSetAndWaitError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SET_AND_WAIT_OLD)
        .data_size(size_of::<[u32; 3]>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for [u32; 3].
    unsafe {
        let data_ptr = req.data.as_ptr().cast::<u32>().cast_mut();
        ptr::write_unaligned(data_ptr, module.as_raw());
        ptr::write_unaligned(data_ptr.add(1), freq_hz);
        ptr::write_unaligned(data_ptr.add(2), timeout as u32);
    }

    ipc::send_sync_request(session).map_err(RequestSetAndWaitError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(RequestSetAndWaitError::ParseResponse)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// request_get (2.0.0+)
// ---------------------------------------------------------------------------

/// Gets the current frequency in Hz (2.0.0+). Keyed by request ID.
pub fn request_get(session: SessionHandle, request_id: u32) -> Result<u32, RequestGetError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET)
        .data_size(size_of::<u32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), request_id);
    }

    ipc::send_sync_request(session).map_err(RequestGetError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u32>()) }
        .map_err(RequestGetError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u32.
    let freq_hz = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(freq_hz)
}

/// Error returned by [`request_get`] / [`request_get_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum RequestGetError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

// ---------------------------------------------------------------------------
// request_get_legacy (pre-2.0.0)
// ---------------------------------------------------------------------------

/// Gets the current frequency in Hz (legacy, pre-2.0.0). Keyed by module ID.
pub fn request_get_legacy(
    session: SessionHandle,
    module: MmuModuleId,
) -> Result<u32, RequestGetError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_OLD)
        .data_size(size_of::<u32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), module.as_raw());
    }

    ipc::send_sync_request(session).map_err(RequestGetError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u32>()) }
        .map_err(RequestGetError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u32.
    let freq_hz = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(freq_hz)
}
