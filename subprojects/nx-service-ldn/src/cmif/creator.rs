//! `ldn:u` / `ldn:s` / `ldn:m` *creator* commands — used to spawn the actual
//! sub-objects (LocalCommunicationService, IClientProcessMonitor,
//! IMonitorService) from the creator session.

use nx_sf::{
    ipc::Handle as RawSessionHandle,
    service::{DispatchError, DomainRef, OwnedSessionHandle, Session},
};

use crate::proto::{CMD_CREATE_CLIENT_PROCESS_MONITOR, CMD_CREATE_SERVICE};

/// Invokes `CreateUserLocalCommService` / `CreateSystemLocalCommService`
/// (cmd 0) on the converted-to-domain `ldn:u`/`ldn:s` creator.
///
/// Returns the raw domain sub-object id assigned by the server. The close
/// obligation is handed on rather than discharged, so the server-side object
/// outlives this call; callers re-address the id per request via
/// [`SessionGuard::open_object_unchecked`](crate::session::SessionGuard::open_object_unchecked).
pub(crate) fn create_service_domain(creator: DomainRef<'_>) -> Result<u32, CreateServiceError> {
    // SAFETY: one IpcBuffer token per thread; IPC is serialized per thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = creator
        .dispatch(CMD_CREATE_SERVICE)
        .out_objects(1)
        .send(&mut buf)
        .map_err(CreateServiceError::Dispatch)?;

    let object = result
        .take_object(0)
        .ok_or(CreateServiceError::MissingObject)?;
    Ok(object.into_raw_object_id())
}

/// Invokes `CreateMonitorService` (cmd 0) on the non-domain `ldn:m` creator.
///
/// Returns a move handle to the freshly-allocated IMonitorService session.
pub(crate) fn create_service_session(
    creator: &Session,
) -> Result<OwnedSessionHandle, CreateServiceError> {
    // SAFETY: one IpcBuffer token per thread; IPC is serialized per thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = creator
        .dispatch(CMD_CREATE_SERVICE)
        .send(&mut buf)
        .map_err(CreateServiceError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(CreateServiceError::MissingObject);
    }
    // SAFETY: the kernel returned a valid session handle in the move-handle slot.
    Ok(OwnedSessionHandle::from_handle_unchecked(
        RawSessionHandle::from_raw_unchecked(result.move_handles[0]),
    ))
}

/// Error returned by [`create_service_domain`] / [`create_service_session`].
#[derive(Debug, thiserror::Error)]
pub enum CreateServiceError {
    /// IPC dispatch failed.
    #[error("failed to dispatch CreateService")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected sub-object / move handle.
    #[error("CreateService response did not include the expected sub-object")]
    MissingObject,
}

/// Invokes `CreateClientProcessMonitor` (cmd 1, `[18.0.0+]`) on the
/// `ldn:u`/`ldn:s` creator domain. Returns the raw ICPM domain sub-object id;
/// the close obligation is handed on rather than discharged, so the
/// server-side object outlives this call.
pub(crate) fn create_client_process_monitor(
    creator: DomainRef<'_>,
) -> Result<u32, CreateClientProcessMonitorError> {
    // SAFETY: one IpcBuffer token per thread; IPC is serialized per thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = creator
        .dispatch(CMD_CREATE_CLIENT_PROCESS_MONITOR)
        .out_objects(1)
        .send(&mut buf)
        .map_err(CreateClientProcessMonitorError::Dispatch)?;

    let object = result
        .take_object(0)
        .ok_or(CreateClientProcessMonitorError::MissingObject)?;
    Ok(object.into_raw_object_id())
}

/// Error returned by [`create_client_process_monitor`].
#[derive(Debug, thiserror::Error)]
pub enum CreateClientProcessMonitorError {
    /// IPC dispatch failed. Likely cause on pre-`[18.0.0]` firmware:
    /// `IncompatSysVer`-style code; the caller should gate on hosversion.
    #[error("failed to dispatch CreateClientProcessMonitor")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected sub-object id.
    #[error("CreateClientProcessMonitor response did not include a sub-object")]
    MissingObject,
}
