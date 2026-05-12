//! `ldn:u` / `ldn:s` / `ldn:m` *creator* commands — used to spawn the actual
//! sub-objects (LocalCommunicationService, IClientProcessMonitor,
//! IMonitorService) from the creator session.

use nx_sf::service::{DispatchError, Domain, Session};
use nx_svc::ipc::Handle as SessionHandle;

use crate::proto::{CMD_CREATE_CLIENT_PROCESS_MONITOR, CMD_CREATE_SERVICE};

/// Invokes `CreateUserLocalCommService` / `CreateSystemLocalCommService`
/// (cmd 0) on the converted-to-domain `ldn:u`/`ldn:s` creator.
///
/// Returns the raw domain sub-object id assigned by the server.
pub(crate) fn create_service_domain(creator: &Domain) -> Result<u32, CreateServiceError> {
    let result = creator
        .dispatch(CMD_CREATE_SERVICE)
        .out_objects(1)
        .send()
        .map_err(CreateServiceError::Dispatch)?;

    if result.objects.is_empty() {
        return Err(CreateServiceError::MissingObject);
    }
    Ok(result.objects[0])
}

/// Invokes `CreateMonitorService` (cmd 0) on the non-domain `ldn:m` creator.
///
/// Returns a move handle to the freshly-allocated IMonitorService session.
pub(crate) fn create_service_session(
    creator: &Session,
) -> Result<SessionHandle, CreateServiceError> {
    let result = creator
        .dispatch(CMD_CREATE_SERVICE)
        .send()
        .map_err(CreateServiceError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(CreateServiceError::MissingObject);
    }
    // SAFETY: the kernel returned a valid session handle in the move-handle slot.
    Ok(unsafe { SessionHandle::from_raw(result.move_handles[0]) })
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
/// `ldn:u`/`ldn:s` creator domain. Returns the raw ICPM domain sub-object id.
pub(crate) fn create_client_process_monitor(
    creator: &Domain,
) -> Result<u32, CreateClientProcessMonitorError> {
    let result = creator
        .dispatch(CMD_CREATE_CLIENT_PROCESS_MONITOR)
        .out_objects(1)
        .send()
        .map_err(CreateClientProcessMonitorError::Dispatch)?;

    if result.objects.is_empty() {
        return Err(CreateClientProcessMonitorError::MissingObject);
    }
    Ok(result.objects[0])
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
