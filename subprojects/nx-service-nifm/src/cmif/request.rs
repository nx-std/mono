//! `IRequest` (sub-object of `IGeneralService`) CMIF dispatch helpers.
//!
//! Mirrors libnx's `nifmRequest*` surface. The crate is hosversion unaware —
//! the caller must avoid [`set_kept_in_sleep`] /
//! [`register_socket_descriptor`] / [`unregister_socket_descriptor`] on
//! pre-`[3.0.0]` firmware.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainObjectRef,
    DomainTarget,
    OutHandleAttr,
};
use nx_svc::sync::EventHandle;
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::{
        dispatch_in,
        dispatch_no_io,
        dispatch_out,
    },
    proto::{
        CMD_REQ_CANCEL,
        CMD_REQ_GET_APPLET_INFO,
        CMD_REQ_GET_REQUEST_STATE,
        CMD_REQ_GET_RESULT,
        CMD_REQ_GET_SYSTEM_EVENT_READABLE_HANDLES,
        CMD_REQ_REGISTER_SOCKET_DESCRIPTOR,
        CMD_REQ_SET_KEPT_IN_SLEEP,
        CMD_REQ_SET_NETWORK_PROFILE_ID,
        CMD_REQ_SUBMIT,
        CMD_REQ_UNREGISTER_SOCKET_DESCRIPTOR,
    },
    types::{
        AppletInfo,
        SocketFd,
        Uuid,
    },
};

/// `GetRequestState` (cmd 0). Returns the raw `u32` state without validating it
/// against [`crate::proto::NifmRequestState`].
pub(crate) fn get_request_state_raw(object: DomainObjectRef<'_>) -> Result<u32, DispatchError> {
    dispatch_out::<u32>(object, CMD_REQ_GET_REQUEST_STATE)
}

/// `GetResult` (cmd 1). The CMIF result code *is* the Switch-side `Result` value.
/// Maps `Ok(())` to "request succeeded" and any [`DispatchError`] to libnx's
/// `nifmGetResult` return code.
pub(crate) fn get_result(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(object, CMD_REQ_GET_RESULT)
}

/// `GetSystemEventReadableHandles` (cmd 2). Returns two copy-handle slots:
/// `event_request_state` (server marks autoclear=true) and `event1`.
pub(crate) fn get_system_event_readable_handles(
    object: DomainObjectRef<'_>,
) -> Result<(EventHandle, EventHandle), GetSystemEventHandlesError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(CMD_REQ_GET_SYSTEM_EVENT_READABLE_HANDLES)
        .out_handle(0, OutHandleAttr::Copy)
        .out_handle(1, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(GetSystemEventHandlesError::Dispatch)?;

    if result.copy_handles.len() < 2 {
        return Err(GetSystemEventHandlesError::MissingHandles);
    }
    // SAFETY: the kernel returned valid event handles in both copy slots.
    let event_request_state = EventHandle::from_raw_unchecked(result.copy_handles[0]);
    let event1 = EventHandle::from_raw_unchecked(result.copy_handles[1]);
    Ok((event_request_state, event1))
}

/// Error returned by [`get_system_event_readable_handles`].
#[derive(Debug, thiserror::Error)]
pub enum GetSystemEventHandlesError {
    /// CMIF dispatch failed.
    #[error("failed to dispatch GetSystemEventReadableHandles")]
    Dispatch(#[source] DispatchError),
    /// Response did not include both expected copy-handles.
    #[error("GetSystemEventReadableHandles response did not include both event handles")]
    MissingHandles,
}

/// `Cancel` (cmd 3).
pub(crate) fn cancel(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(object, CMD_REQ_CANCEL)
}

/// `Submit` (cmd 4).
pub(crate) fn submit(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(object, CMD_REQ_SUBMIT)
}

/// `SetNetworkProfileId` (cmd 9).
pub(crate) fn set_network_profile_id(
    object: DomainObjectRef<'_>,
    uuid: Uuid,
) -> Result<(), DispatchError> {
    dispatch_in(object, CMD_REQ_SET_NETWORK_PROFILE_ID, uuid)
}

/// `GetAppletInfo` (cmd 21). Used by `nifmLaHandleNetworkRequestResult` in libnx.
///
/// The caller supplies a HipcMapAlias output buffer for the variable-length
/// storage data; on success, [`AppletInfo::out_size`] records how many bytes
/// the server actually wrote.
pub(crate) fn get_applet_info(
    object: DomainObjectRef<'_>,
    theme_color: u32,
    buffer: &mut [u8],
) -> Result<AppletInfo, DispatchError> {
    #[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
    #[repr(C)]
    struct Out {
        applet_id: u32,
        mode: u32,
        out_size: u32,
    }
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(CMD_REQ_GET_APPLET_INFO)
        .in_raw(theme_color.as_bytes())
        .out_size(size_of::<Out>())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    let out = result.value::<Out>();
    Ok(AppletInfo {
        applet_id: out.applet_id,
        mode: out.mode,
        out_size: out.out_size,
    })
}

/// `SetKeptInSleep` (cmd 23). Caller must guard on `[3.0.0+]`.
pub(crate) fn set_kept_in_sleep(
    object: DomainObjectRef<'_>,
    flag: bool,
) -> Result<(), DispatchError> {
    let raw: u8 = if flag { 1 } else { 0 };
    dispatch_in(object, CMD_REQ_SET_KEPT_IN_SLEEP, raw)
}

/// `RegisterSocketDescriptor` (cmd 24). Caller must guard on `[3.0.0+]`.
pub(crate) fn register_socket_descriptor<'d>(
    object: impl DomainTarget<'d>,
    sockfd: impl Into<SocketFd>,
) -> Result<(), DispatchError> {
    let sockfd = sockfd.into();
    dispatch_in(object, CMD_REQ_REGISTER_SOCKET_DESCRIPTOR, sockfd)
}

/// `UnregisterSocketDescriptor` (cmd 25). Caller must guard on `[3.0.0+]`.
pub(crate) fn unregister_socket_descriptor<'d>(
    object: impl DomainTarget<'d>,
    sockfd: impl Into<SocketFd>,
) -> Result<(), DispatchError> {
    let sockfd = sockfd.into();
    dispatch_in(object, CMD_REQ_UNREGISTER_SOCKET_DESCRIPTOR, sockfd)
}
