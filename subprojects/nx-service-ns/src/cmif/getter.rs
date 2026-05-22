//! Getter interface commands — obtain sub-interface sessions from ns:am2.

use nx_sf::service::{DispatchError, Session};

use crate::proto;

/// Dispatches a getter command that returns a move handle for a sub-interface.
fn get_interface(service: &Session, cmd_id: u32) -> Result<u32, GetInterfaceError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(cmd_id)
        .send(&mut ipc_buf)
        .map_err(GetInterfaceError::Dispatch)?;

    let Some(handle) = result.move_handles.first().copied() else {
        return Err(GetInterfaceError::MissingHandle);
    };

    Ok(handle)
}

#[inline]
pub(crate) fn get_dynamic_rights_interface(service: &Session) -> Result<u32, GetInterfaceError> {
    get_interface(service, proto::GET_DYNAMIC_RIGHTS_INTERFACE)
}

#[inline]
pub(crate) fn get_readonly_application_control_data_interface(
    service: &Session,
) -> Result<u32, GetInterfaceError> {
    get_interface(
        service,
        proto::GET_READONLY_APPLICATION_CONTROL_DATA_INTERFACE,
    )
}

#[inline]
pub(crate) fn get_readonly_application_record_interface(
    service: &Session,
) -> Result<u32, GetInterfaceError> {
    get_interface(service, proto::GET_READONLY_APPLICATION_RECORD_INTERFACE)
}

#[inline]
pub(crate) fn get_ecommerce_interface(service: &Session) -> Result<u32, GetInterfaceError> {
    get_interface(service, proto::GET_ECOMMERCE_INTERFACE)
}

#[inline]
pub(crate) fn get_application_version_interface(
    service: &Session,
) -> Result<u32, GetInterfaceError> {
    get_interface(service, proto::GET_APPLICATION_VERSION_INTERFACE)
}

#[inline]
pub(crate) fn get_factory_reset_interface(service: &Session) -> Result<u32, GetInterfaceError> {
    get_interface(service, proto::GET_FACTORY_RESET_INTERFACE)
}

#[inline]
pub(crate) fn get_account_proxy_interface(service: &Session) -> Result<u32, GetInterfaceError> {
    get_interface(service, proto::GET_ACCOUNT_PROXY_INTERFACE)
}

#[inline]
pub(crate) fn get_application_manager_interface(
    service: &Session,
) -> Result<u32, GetInterfaceError> {
    get_interface(service, proto::GET_APPLICATION_MANAGER_INTERFACE)
}

#[inline]
pub(crate) fn get_download_task_interface(service: &Session) -> Result<u32, GetInterfaceError> {
    get_interface(service, proto::GET_DOWNLOAD_TASK_INTERFACE)
}

#[inline]
pub(crate) fn get_content_management_interface(
    service: &Session,
) -> Result<u32, GetInterfaceError> {
    get_interface(service, proto::GET_CONTENT_MANAGEMENT_INTERFACE)
}

#[inline]
pub(crate) fn get_document_interface(service: &Session) -> Result<u32, GetInterfaceError> {
    get_interface(service, proto::GET_DOCUMENT_INTERFACE)
}

/// Error returned by getter interface commands.
#[derive(Debug, thiserror::Error)]
pub enum GetInterfaceError {
    #[error("failed to dispatch getter command")]
    Dispatch(#[source] DispatchError),
    #[error("getter response did not include expected move handle")]
    MissingHandle,
}
