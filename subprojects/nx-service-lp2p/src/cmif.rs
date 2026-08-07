//! CMIF protocol operations for the LP2P service.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainObjectRef,
    DomainRef,
    OutHandleAttr,
    Session,
};
use zerocopy::IntoBytes as _;

use crate::{
    proto,
    types::{
        CreateNetworkServiceIn,
        GetAdvertiseDataOut,
        Lp2pGroupId,
        Lp2pGroupInfo,
        Lp2pIpConfig,
        Lp2pMacAddress,
        Lp2pNodeInfo,
        Lp2pScanResult,
        RecvFromOtherGroupOut,
        SendToOtherGroupIn,
    },
};

/// Creates an INetworkService sub-object (cmd 0).
///
/// Returns the raw sub-object ID for the new `INetworkService` domain object.
/// The close obligation is handed on rather than discharged: the caller
/// re-addresses the id through the long-lived parent domain.
pub(crate) fn create_network_service(
    domain: DomainRef<'_>,
    inval: u32,
) -> Result<u32, CreateNetworkServiceError> {
    let input = CreateNetworkServiceIn {
        inval,
        _pad: 0,
        pid_placeholder: 0,
    };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let mut result = domain
        .dispatch(proto::CREATE_NETWORK_SERVICE)
        .in_raw(input.as_bytes())
        .send_pid()
        .out_objects(1)
        .send(&mut buf)
        .map_err(CreateNetworkServiceError::Dispatch)?;

    let object = result
        .take_object(0)
        .ok_or(CreateNetworkServiceError::MissingObject)?;
    Ok(object.into_raw_object_id())
}

/// Creates an INetworkServiceMonitor sub-object (cmd 8, non-domain).
///
/// Returns the monitor's session handle (move handle).
pub(crate) fn create_network_service_monitor(session: &Session) -> Result<u32, CreateMonitorError> {
    let pid_placeholder: u64 = 0;
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = session
        .dispatch(proto::CREATE_NETWORK_SERVICE_MONITOR)
        .in_raw(pid_placeholder.as_bytes())
        .send_pid()
        .send(&mut buf)
        .map_err(CreateMonitorError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(CreateMonitorError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

/// Scans for groups (cmd 512).
pub(crate) fn scan(
    object: DomainObjectRef<'_>,
    info: &Lp2pGroupInfo,
    results: &mut [Lp2pScanResult],
) -> Result<i32, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(proto::SCAN)
        .out_size(size_of::<i32>())
        .in_buffer(
            info.as_bytes(),
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .out_buffer(results.as_mut_bytes(), BufferAttr::HIPC_AUTO_SELECT)
        .send(&mut buf)?;

    Ok(*result.value::<i32>())
}

/// Creates a group (cmd 768).
pub(crate) fn create_group(
    object: DomainObjectRef<'_>,
    info: &Lp2pGroupInfo,
) -> Result<(), DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::CREATE_GROUP)
        .in_buffer(
            info.as_bytes(),
            BufferAttr::FIXED_SIZE.or(BufferAttr::HIPC_AUTO_SELECT),
        )
        .send(&mut buf)
        .map(|_| ())
}

/// Destroys the current group (cmd 776).
pub(crate) fn destroy_group(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::DESTROY_GROUP)
        .send(&mut buf)
        .map(|_| ())
}

/// Sets advertise data (cmd 784).
pub(crate) fn set_advertise_data(
    object: DomainObjectRef<'_>,
    data: &[u8],
) -> Result<(), DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::SET_ADVERTISE_DATA)
        .in_buffer(data, BufferAttr::HIPC_AUTO_SELECT)
        .send(&mut buf)
        .map(|_| ())
}

/// Sends data to another group (cmd 1536).
#[allow(clippy::too_many_arguments)]
pub(crate) fn send_to_other_group(
    object: DomainObjectRef<'_>,
    data: &[u8],
    addr: Lp2pMacAddress,
    group_id: Lp2pGroupId,
    frequency: i16,
    channel: i16,
    flags: u32,
) -> Result<(), DispatchError> {
    let input = SendToOtherGroupIn {
        addr,
        group_id,
        frequency,
        channel,
        flags,
    };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::SEND_TO_OTHER_GROUP)
        .in_raw(input.as_bytes())
        .in_buffer(data, BufferAttr::HIPC_AUTO_SELECT)
        .send(&mut buf)
        .map(|_| ())
}

/// Receives data from another group (cmd 1544).
pub(crate) fn recv_from_other_group(
    object: DomainObjectRef<'_>,
    flags: u32,
    buffer: &mut [u8],
) -> Result<RecvFromOtherGroupOut, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(proto::RECV_FROM_OTHER_GROUP)
        .in_raw(flags.as_bytes())
        .out_size(size_of::<RecvFromOtherGroupOut>())
        .out_buffer(buffer, BufferAttr::HIPC_AUTO_SELECT)
        .send(&mut buf)?;

    Ok(*result.value::<RecvFromOtherGroupOut>())
}

/// Adds an acceptable group ID (cmd 1552).
pub(crate) fn add_acceptable_group_id(
    object: DomainObjectRef<'_>,
    group_id: Lp2pGroupId,
) -> Result<(), DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::ADD_ACCEPTABLE_GROUP_ID)
        .in_raw(group_id.as_bytes())
        .send(&mut buf)
        .map(|_| ())
}

/// Removes the acceptable group ID (cmd 1560).
pub(crate) fn remove_acceptable_group_id(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::REMOVE_ACCEPTABLE_GROUP_ID)
        .send(&mut buf)
        .map(|_| ())
}

/// Attaches the network interface state change event (cmd 256).
pub(crate) fn attach_network_interface_state_change_event(
    session: &Session,
) -> Result<u32, AttachEventError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = session
        .dispatch(proto::ATTACH_NETWORK_INTERFACE_STATE_CHANGE_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)
        .map_err(AttachEventError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(AttachEventError::MissingHandle);
    }

    Ok(result.copy_handles[0])
}

/// Gets the last network interface error (cmd 264).
pub(crate) fn get_network_interface_last_error(session: &Session) -> Result<(), DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    session
        .dispatch(proto::GET_NETWORK_INTERFACE_LAST_ERROR)
        .send(&mut buf)
        .map(|_| ())
}

/// Gets the current role (cmd 272).
pub(crate) fn get_role(session: &Session) -> Result<u8, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = session
        .dispatch(proto::GET_ROLE)
        .out_size(size_of::<u8>())
        .send(&mut buf)?;

    Ok(*result.value::<u8>())
}

/// Gets advertise data (shared for cmds 280/281).
pub(crate) fn get_advertise_data(
    session: &Session,
    cmd_id: u32,
    buffer: &mut [u8],
) -> Result<GetAdvertiseDataOut, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = session
        .dispatch(cmd_id)
        .out_size(size_of::<GetAdvertiseDataOut>())
        .out_buffer(buffer, BufferAttr::HIPC_AUTO_SELECT)
        .send(&mut buf)?;

    Ok(*result.value::<GetAdvertiseDataOut>())
}

/// Gets the current group info (cmd 288).
pub(crate) fn get_group_info(
    session: &Session,
    out: &mut Lp2pGroupInfo,
) -> Result<(), DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    session
        .dispatch(proto::GET_GROUP_INFO)
        .out_buffer(
            out.as_mut_bytes(),
            BufferAttr::FIXED_SIZE.or(BufferAttr::HIPC_AUTO_SELECT),
        )
        .send(&mut buf)
        .map(|_| ())
}

/// Joins a group (cmd 296).
pub(crate) fn join(
    session: &Session,
    out: &mut Lp2pGroupInfo,
    info: &Lp2pGroupInfo,
) -> Result<(), DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    session
        .dispatch(proto::JOIN)
        .out_buffer(
            out.as_mut_bytes(),
            BufferAttr::HIPC_AUTO_SELECT.or(BufferAttr::FIXED_SIZE),
        )
        .in_buffer(
            info.as_bytes(),
            BufferAttr::HIPC_AUTO_SELECT.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut buf)
        .map(|_| ())
}

/// Gets the group owner (cmd 304).
pub(crate) fn get_group_owner(session: &Session) -> Result<Lp2pNodeInfo, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = session
        .dispatch(proto::GET_GROUP_OWNER)
        .out_size(size_of::<Lp2pNodeInfo>())
        .send(&mut buf)?;

    Ok(*result.value::<Lp2pNodeInfo>())
}

/// Gets the IP configuration (cmd 312).
pub(crate) fn get_ip_config(
    session: &Session,
    out: &mut Lp2pIpConfig,
) -> Result<(), DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    session
        .dispatch(proto::GET_IP_CONFIG)
        .out_buffer(
            out.as_mut_bytes(),
            BufferAttr::FIXED_SIZE.or(BufferAttr::HIPC_POINTER),
        )
        .send(&mut buf)
        .map(|_| ())
}

/// Leaves the current group (cmd 320).
pub(crate) fn leave(session: &Session) -> Result<u32, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = session
        .dispatch(proto::LEAVE)
        .out_size(size_of::<u32>())
        .send(&mut buf)?;

    Ok(*result.value::<u32>())
}

/// Attaches the join event (cmd 328).
pub(crate) fn attach_join_event(session: &Session) -> Result<u32, AttachEventError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = session
        .dispatch(proto::ATTACH_JOIN_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)
        .map_err(AttachEventError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(AttachEventError::MissingHandle);
    }

    Ok(result.copy_handles[0])
}

/// Gets group members (cmd 336).
pub(crate) fn get_members(
    session: &Session,
    members: &mut [Lp2pNodeInfo],
) -> Result<i32, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = session
        .dispatch(proto::GET_MEMBERS)
        .out_size(size_of::<i32>())
        .out_buffer(members.as_mut_bytes(), BufferAttr::HIPC_AUTO_SELECT)
        .send(&mut buf)?;

    Ok(*result.value::<i32>())
}

/// Errors from [`create_network_service`].
#[derive(Debug, thiserror::Error)]
pub enum CreateNetworkServiceError {
    /// The underlying IPC dispatch for `CreateNetworkService` failed.
    ///
    /// Reported when sending the CMIF request or parsing the response header
    /// fails (kernel rejection, malformed reply, etc.).
    #[error("IPC dispatch failed for CreateNetworkService")]
    Dispatch(#[source] DispatchError),
    /// The server replied successfully but did not include the requested
    /// `INetworkService` domain sub-object in the response.
    ///
    /// Treated as a protocol violation: the call asked for one out object,
    /// so a missing slot leaves the caller without a usable handle.
    #[error("CreateNetworkService returned no domain sub-object")]
    MissingObject,
}

/// Errors from [`create_network_service_monitor`].
#[derive(Debug, thiserror::Error)]
pub enum CreateMonitorError {
    #[error("IPC dispatch failed for CreateNetworkServiceMonitor")]
    Dispatch(#[source] DispatchError),
    #[error("CreateNetworkServiceMonitor returned no move handle")]
    MissingHandle,
}

/// Errors from event attachment commands.
#[derive(Debug, thiserror::Error)]
pub enum AttachEventError {
    #[error("IPC dispatch failed for event attachment")]
    Dispatch(#[source] DispatchError),
    #[error("event attachment returned no copy handle")]
    MissingHandle,
}
