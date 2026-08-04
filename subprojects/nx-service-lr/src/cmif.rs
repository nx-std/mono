//! CMIF protocol operations for the location resolver service.

use core::mem::size_of;

use nx_sf::service::{
    DispatchError,
    Session,
};

use crate::{
    dispatch,
    proto,
    types::LR_MAX_PATH,
};

/// Opens a location resolver sub-object for the given storage.
///
/// Returns the move handle for the new `ILocationResolver` session.
pub(crate) fn open_location_resolver(
    service: &Session,
    storage: u8,
) -> Result<u32, OpenLocationResolverError> {
    // SAFETY: `storage` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const storage).cast::<u8>(), size_of::<u8>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::OPEN_LOCATION_RESOLVER)
        .in_raw(in_bytes)
        .send(&mut ipc_buf)
        .map_err(OpenLocationResolverError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(OpenLocationResolverError::MissingHandle);
    }
    Ok(result.move_handles[0])
}

/// Opens a registered location resolver sub-object.
///
/// Returns the move handle for the new `IRegisteredLocationResolver` session.
pub(crate) fn open_registered_location_resolver(
    service: &Session,
) -> Result<u32, OpenRegisteredLocationResolverError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::OPEN_REGISTERED_LOCATION_RESOLVER)
        .send(&mut ipc_buf)
        .map_err(OpenRegisteredLocationResolverError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(OpenRegisteredLocationResolverError::MissingHandle);
    }
    Ok(result.move_handles[0])
}

// -- ILocationResolver commands --

pub(crate) fn resolve_program_path(
    service: &Session,
    tid: u64,
    out: &mut [u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    dispatch::resolve_path(service, proto::RESOLVE_PROGRAM_PATH, tid, out)
}

pub(crate) fn redirect_program_path(
    service: &Session,
    tid: u64,
    path: &[u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    dispatch::redirect_path(service, proto::REDIRECT_PROGRAM_PATH, tid, path)
}

pub(crate) fn resolve_application_control_path(
    service: &Session,
    tid: u64,
    out: &mut [u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    dispatch::resolve_path(service, proto::RESOLVE_APPLICATION_CONTROL_PATH, tid, out)
}

pub(crate) fn resolve_application_html_document_path(
    service: &Session,
    tid: u64,
    out: &mut [u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    dispatch::resolve_path(
        service,
        proto::RESOLVE_APPLICATION_HTML_DOCUMENT_PATH,
        tid,
        out,
    )
}

pub(crate) fn resolve_data_path(
    service: &Session,
    tid: u64,
    out: &mut [u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    dispatch::resolve_path(service, proto::RESOLVE_DATA_PATH, tid, out)
}

pub(crate) fn redirect_application_control_path_legacy(
    service: &Session,
    tid: u64,
    path: &[u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    dispatch::redirect_path(service, proto::REDIRECT_APPLICATION_CONTROL_PATH, tid, path)
}

pub(crate) fn redirect_application_control_path(
    service: &Session,
    tid: u64,
    tid2: u64,
    path: &[u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    dispatch::redirect_application_path(
        service,
        proto::REDIRECT_APPLICATION_CONTROL_PATH,
        tid,
        tid2,
        path,
    )
}

pub(crate) fn redirect_application_html_document_path_legacy(
    service: &Session,
    tid: u64,
    path: &[u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    dispatch::redirect_path(
        service,
        proto::REDIRECT_APPLICATION_HTML_DOCUMENT_PATH,
        tid,
        path,
    )
}

pub(crate) fn redirect_application_html_document_path(
    service: &Session,
    tid: u64,
    tid2: u64,
    path: &[u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    dispatch::redirect_application_path(
        service,
        proto::REDIRECT_APPLICATION_HTML_DOCUMENT_PATH,
        tid,
        tid2,
        path,
    )
}

pub(crate) fn resolve_application_legal_information_path(
    service: &Session,
    tid: u64,
    out: &mut [u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    dispatch::resolve_path(
        service,
        proto::RESOLVE_APPLICATION_LEGAL_INFORMATION_PATH,
        tid,
        out,
    )
}

pub(crate) fn redirect_application_legal_information_path_legacy(
    service: &Session,
    tid: u64,
    path: &[u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    dispatch::redirect_path(
        service,
        proto::REDIRECT_APPLICATION_LEGAL_INFORMATION_PATH,
        tid,
        path,
    )
}

pub(crate) fn redirect_application_legal_information_path(
    service: &Session,
    tid: u64,
    tid2: u64,
    path: &[u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    dispatch::redirect_application_path(
        service,
        proto::REDIRECT_APPLICATION_LEGAL_INFORMATION_PATH,
        tid,
        tid2,
        path,
    )
}

pub(crate) fn refresh(service: &Session) -> Result<(), DispatchError> {
    dispatch::dispatch_no_io(service, proto::REFRESH)
}

pub(crate) fn erase_program_redirection(service: &Session, tid: u64) -> Result<(), DispatchError> {
    dispatch::dispatch_in_u64(service, proto::ERASE_PROGRAM_REDIRECTION, tid)
}

// -- IRegisteredLocationResolver commands --

pub(crate) fn reg_resolve_program_path(
    service: &Session,
    tid: u64,
    out: &mut [u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    dispatch::resolve_path(service, proto::REG_RESOLVE_PROGRAM_PATH, tid, out)
}

/// Error returned by [`open_location_resolver`].
#[derive(Debug, thiserror::Error)]
pub enum OpenLocationResolverError {
    /// IPC dispatch failed.
    #[error("failed to dispatch OpenLocationResolver")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected move handle.
    #[error("OpenLocationResolver response did not include the expected session handle")]
    MissingHandle,
}

/// Error returned by [`open_registered_location_resolver`].
#[derive(Debug, thiserror::Error)]
pub enum OpenRegisteredLocationResolverError {
    /// IPC dispatch failed.
    #[error("failed to dispatch OpenRegisteredLocationResolver")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected move handle.
    #[error("OpenRegisteredLocationResolver response did not include the expected session handle")]
    MissingHandle,
}
