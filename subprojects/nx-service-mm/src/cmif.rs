//! CMIF protocol operations for the multimedia service.

use nx_sf::{
    cmif,
    ipc::{self, Handle as SessionHandle},
};

use crate::{proto, types::MmuModuleId};

#[repr(C, packed)]
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
struct InitializeIn {
    module: u32,
    unk: u32,
    autoclear: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
struct SetAndWaitIn {
    key: u32,
    freq_hz: u32,
    timeout: u32,
}

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

    let input = InitializeIn {
        module: module.as_raw(),
        unk,
        autoclear: autoclear as u32,
    };
    let req = cmif::CmifRequestBuilder::new(proto::INITIALIZE)
        .with_data_value(&input)
        .build();
    req.write_to(&mut buf)
        .map_err(RequestInitializeError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(RequestInitializeError::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(RequestInitializeError::ParseResponse)?;
    let id = *resp.payload;

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
    ParseResponse(#[source] cmif::ParseError),
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

    let input = InitializeIn {
        module: module.as_raw(),
        unk,
        autoclear: autoclear as u32,
    };
    let req = cmif::CmifRequestBuilder::new(proto::INITIALIZE_OLD)
        .with_data_value(&input)
        .build();
    req.write_to(&mut buf)
        .map_err(RequestInitializeError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(RequestInitializeError::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(RequestInitializeError::ParseResponse)?;
    let id = *resp.payload;

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
        .with_data_value(&request_id)
        .build();
    req.write_to(&mut buf)
        .map_err(RequestFinalizeError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(RequestFinalizeError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(RequestFinalizeError::ParseResponse)?;

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
    ParseResponse(#[source] cmif::ParseError),
}

/// Finalises a multimedia request (legacy, pre-2.0.0). Keyed by module ID.
pub fn request_finalize_legacy(
    session: SessionHandle,
    module: MmuModuleId,
) -> Result<(), RequestFinalizeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let module_raw = module.as_raw();
    let req = cmif::CmifRequestBuilder::new(proto::FINALIZE_OLD)
        .with_data_value(&module_raw)
        .build();
    req.write_to(&mut buf)
        .map_err(RequestFinalizeError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(RequestFinalizeError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(RequestFinalizeError::ParseResponse)?;

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

    let input = SetAndWaitIn {
        key: request_id,
        freq_hz,
        timeout: timeout as u32,
    };
    let req = cmif::CmifRequestBuilder::new(proto::SET_AND_WAIT)
        .with_data_value(&input)
        .build();
    req.write_to(&mut buf)
        .map_err(RequestSetAndWaitError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(RequestSetAndWaitError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(RequestSetAndWaitError::ParseResponse)?;

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
    ParseResponse(#[source] cmif::ParseError),
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

    let input = SetAndWaitIn {
        key: module.as_raw(),
        freq_hz,
        timeout: timeout as u32,
    };
    let req = cmif::CmifRequestBuilder::new(proto::SET_AND_WAIT_OLD)
        .with_data_value(&input)
        .build();
    req.write_to(&mut buf)
        .map_err(RequestSetAndWaitError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(RequestSetAndWaitError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(RequestSetAndWaitError::ParseResponse)?;

    Ok(())
}

/// Gets the current frequency in Hz (2.0.0+). Keyed by request ID.
pub fn request_get(session: SessionHandle, request_id: u32) -> Result<u32, RequestGetError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET)
        .with_data_value(&request_id)
        .build();
    req.write_to(&mut buf)
        .map_err(RequestGetError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(RequestGetError::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(RequestGetError::ParseResponse)?;
    let freq_hz = *resp.payload;

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
    ParseResponse(#[source] cmif::ParseError),
}

/// Gets the current frequency in Hz (legacy, pre-2.0.0). Keyed by module ID.
pub fn request_get_legacy(
    session: SessionHandle,
    module: MmuModuleId,
) -> Result<u32, RequestGetError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let module_raw = module.as_raw();
    let req = cmif::CmifRequestBuilder::new(proto::GET_OLD)
        .with_data_value(&module_raw)
        .build();
    req.write_to(&mut buf)
        .map_err(RequestGetError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(RequestGetError::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(RequestGetError::ParseResponse)?;
    let freq_hz = *resp.payload;

    Ok(freq_hz)
}
