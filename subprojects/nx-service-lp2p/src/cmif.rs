//! CMIF protocol operations for the LP2P service.

use core::{mem::size_of, ptr};

use nx_sf::service::{BufferAttr, DispatchError, Domain, DomainObject, OutHandleAttr, Session};

use crate::{
    proto,
    types::{
        CreateNetworkServiceIn, GetAdvertiseDataOut, Lp2pGroupId, Lp2pGroupInfo, Lp2pIpConfig,
        Lp2pMacAddress, Lp2pNodeInfo, Lp2pScanResult, RecvFromOtherGroupOut, SendToOtherGroupIn,
    },
};

// ---------------------------------------------------------------------------
// Root service commands
// ---------------------------------------------------------------------------

/// Creates an INetworkService sub-object (cmd 0).
///
/// Returns the raw sub-object ID for the new `INetworkService` domain object.
pub(crate) fn create_network_service(domain: &Domain, inval: u32) -> Result<u32, DispatchError> {
    let input = CreateNetworkServiceIn {
        inval,
        _pad: 0,
        pid_placeholder: 0,
    };

    // SAFETY: `input` lives on the stack until `.send()` returns.
    let result = unsafe {
        domain
            .dispatch(proto::CREATE_NETWORK_SERVICE)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<CreateNetworkServiceIn>(),
            )
            .send_pid()
            .out_objects(1)
            .send()?
    };

    Ok(result.objects[0])
}

/// Creates an INetworkServiceMonitor sub-object (cmd 8, non-domain).
///
/// Returns the monitor's session handle (move handle).
pub(crate) fn create_network_service_monitor(session: &Session) -> Result<u32, CreateMonitorError> {
    let pid_placeholder: u64 = 0;

    // SAFETY: `pid_placeholder` lives on the stack until `.send()` returns.
    let result = unsafe {
        session
            .dispatch(proto::CREATE_NETWORK_SERVICE_MONITOR)
            .in_raw((&raw const pid_placeholder).cast::<u8>(), size_of::<u64>())
            .send_pid()
            .send()
            .map_err(CreateMonitorError::Dispatch)?
    };

    if result.move_handles.is_empty() {
        return Err(CreateMonitorError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

// ---------------------------------------------------------------------------
// INetworkService commands (domain, dispatched through session pool)
// ---------------------------------------------------------------------------

/// Scans for groups (cmd 512).
pub(crate) fn scan(
    object: &DomainObject<'_>,
    info: &Lp2pGroupInfo,
    results: &mut [Lp2pScanResult],
) -> Result<i32, DispatchError> {
    let result = object
        .dispatch(proto::SCAN)
        .out_size(size_of::<i32>())
        .buffer(
            (&raw const *info).cast::<u8>(),
            size_of::<Lp2pGroupInfo>(),
            BufferAttr::HIPC_POINTER
                .or(BufferAttr::IN)
                .or(BufferAttr::FIXED_SIZE),
        )
        .buffer(
            results.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(results),
            BufferAttr::HIPC_AUTO_SELECT.or(BufferAttr::OUT),
        )
        .send()?;

    // SAFETY: response payload is at least size_of::<i32>().
    let total_out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) };
    Ok(total_out)
}

/// Creates a group (cmd 768).
pub(crate) fn create_group(
    object: &DomainObject<'_>,
    info: &Lp2pGroupInfo,
) -> Result<(), DispatchError> {
    object
        .dispatch(proto::CREATE_GROUP)
        .buffer(
            (&raw const *info).cast::<u8>(),
            size_of::<Lp2pGroupInfo>(),
            BufferAttr::FIXED_SIZE
                .or(BufferAttr::HIPC_AUTO_SELECT)
                .or(BufferAttr::IN),
        )
        .send()
        .map(|_| ())
}

/// Destroys the current group (cmd 776).
pub(crate) fn destroy_group(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    object.dispatch(proto::DESTROY_GROUP).send().map(|_| ())
}

/// Sets advertise data (cmd 784).
pub(crate) fn set_advertise_data(
    object: &DomainObject<'_>,
    data: &[u8],
) -> Result<(), DispatchError> {
    object
        .dispatch(proto::SET_ADVERTISE_DATA)
        .buffer(
            data.as_ptr(),
            data.len(),
            BufferAttr::HIPC_AUTO_SELECT.or(BufferAttr::IN),
        )
        .send()
        .map(|_| ())
}

/// Sends data to another group (cmd 1536).
#[allow(clippy::too_many_arguments)]
pub(crate) fn send_to_other_group(
    object: &DomainObject<'_>,
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

    // SAFETY: `input` and `data` live until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::SEND_TO_OTHER_GROUP)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<SendToOtherGroupIn>(),
            )
            .buffer(
                data.as_ptr(),
                data.len(),
                BufferAttr::HIPC_AUTO_SELECT.or(BufferAttr::IN),
            )
            .send()
            .map(|_| ())
    }
}

/// Receives data from another group (cmd 1544).
pub(crate) fn recv_from_other_group(
    object: &DomainObject<'_>,
    flags: u32,
    buffer: &mut [u8],
) -> Result<RecvFromOtherGroupOut, DispatchError> {
    // SAFETY: `flags` and `buffer` live until `.send()` returns.
    let result = unsafe {
        object
            .dispatch(proto::RECV_FROM_OTHER_GROUP)
            .in_raw((&raw const flags).cast::<u8>(), size_of::<u32>())
            .out_size(size_of::<RecvFromOtherGroupOut>())
            .buffer(
                buffer.as_mut_ptr(),
                buffer.len(),
                BufferAttr::HIPC_AUTO_SELECT.or(BufferAttr::OUT),
            )
            .send()?
    };

    // SAFETY: response payload is at least size_of::<RecvFromOtherGroupOut>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<RecvFromOtherGroupOut>()) };
    Ok(out)
}

/// Adds an acceptable group ID (cmd 1552).
pub(crate) fn add_acceptable_group_id(
    object: &DomainObject<'_>,
    group_id: Lp2pGroupId,
) -> Result<(), DispatchError> {
    // SAFETY: `group_id` lives until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::ADD_ACCEPTABLE_GROUP_ID)
            .in_raw((&raw const group_id).cast::<u8>(), size_of::<Lp2pGroupId>())
            .send()
            .map(|_| ())
    }
}

/// Removes the acceptable group ID (cmd 1560).
pub(crate) fn remove_acceptable_group_id(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    object
        .dispatch(proto::REMOVE_ACCEPTABLE_GROUP_ID)
        .send()
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// INetworkServiceMonitor commands (non-domain, direct dispatch)
// ---------------------------------------------------------------------------

/// Attaches the network interface state change event (cmd 256).
pub(crate) fn attach_network_interface_state_change_event(
    session: &Session,
) -> Result<u32, AttachEventError> {
    let result = session
        .dispatch(proto::ATTACH_NETWORK_INTERFACE_STATE_CHANGE_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send()
        .map_err(AttachEventError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(AttachEventError::MissingHandle);
    }

    Ok(result.copy_handles[0])
}

/// Gets the last network interface error (cmd 264).
pub(crate) fn get_network_interface_last_error(session: &Session) -> Result<(), DispatchError> {
    session
        .dispatch(proto::GET_NETWORK_INTERFACE_LAST_ERROR)
        .send()
        .map(|_| ())
}

/// Gets the current role (cmd 272).
pub(crate) fn get_role(session: &Session) -> Result<u8, DispatchError> {
    let result = session
        .dispatch(proto::GET_ROLE)
        .out_size(size_of::<u8>())
        .send()?;

    // SAFETY: response payload is at least size_of::<u8>().
    let role = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    Ok(role)
}

/// Gets advertise data (shared for cmds 280/281).
pub(crate) fn get_advertise_data(
    session: &Session,
    cmd_id: u32,
    buffer: &mut [u8],
) -> Result<GetAdvertiseDataOut, DispatchError> {
    let result = session
        .dispatch(cmd_id)
        .out_size(size_of::<GetAdvertiseDataOut>())
        .buffer(
            buffer.as_mut_ptr(),
            buffer.len(),
            BufferAttr::HIPC_AUTO_SELECT.or(BufferAttr::OUT),
        )
        .send()?;

    // SAFETY: response payload is at least size_of::<GetAdvertiseDataOut>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<GetAdvertiseDataOut>()) };
    Ok(out)
}

/// Gets the current group info (cmd 288).
pub(crate) fn get_group_info(
    session: &Session,
    out: &mut Lp2pGroupInfo,
) -> Result<(), DispatchError> {
    session
        .dispatch(proto::GET_GROUP_INFO)
        .buffer(
            (out as *mut Lp2pGroupInfo).cast::<u8>(),
            size_of::<Lp2pGroupInfo>(),
            BufferAttr::FIXED_SIZE
                .or(BufferAttr::HIPC_AUTO_SELECT)
                .or(BufferAttr::OUT),
        )
        .send()
        .map(|_| ())
}

/// Joins a group (cmd 296).
pub(crate) fn join(
    session: &Session,
    out: &mut Lp2pGroupInfo,
    info: &Lp2pGroupInfo,
) -> Result<(), DispatchError> {
    session
        .dispatch(proto::JOIN)
        .buffer(
            (out as *mut Lp2pGroupInfo).cast::<u8>(),
            size_of::<Lp2pGroupInfo>(),
            BufferAttr::HIPC_AUTO_SELECT
                .or(BufferAttr::OUT)
                .or(BufferAttr::FIXED_SIZE),
        )
        .buffer(
            (&raw const *info).cast::<u8>(),
            size_of::<Lp2pGroupInfo>(),
            BufferAttr::HIPC_AUTO_SELECT
                .or(BufferAttr::IN)
                .or(BufferAttr::FIXED_SIZE),
        )
        .send()
        .map(|_| ())
}

/// Gets the group owner (cmd 304).
pub(crate) fn get_group_owner(session: &Session) -> Result<Lp2pNodeInfo, DispatchError> {
    let result = session
        .dispatch(proto::GET_GROUP_OWNER)
        .out_size(size_of::<Lp2pNodeInfo>())
        .send()?;

    // SAFETY: response payload is at least size_of::<Lp2pNodeInfo>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<Lp2pNodeInfo>()) };
    Ok(out)
}

/// Gets the IP configuration (cmd 312).
pub(crate) fn get_ip_config(
    session: &Session,
    out: &mut Lp2pIpConfig,
) -> Result<(), DispatchError> {
    session
        .dispatch(proto::GET_IP_CONFIG)
        .buffer(
            (out as *mut Lp2pIpConfig).cast::<u8>(),
            size_of::<Lp2pIpConfig>(),
            BufferAttr::FIXED_SIZE
                .or(BufferAttr::HIPC_POINTER)
                .or(BufferAttr::OUT),
        )
        .send()
        .map(|_| ())
}

/// Leaves the current group (cmd 320).
pub(crate) fn leave(session: &Session) -> Result<u32, DispatchError> {
    let result = session
        .dispatch(proto::LEAVE)
        .out_size(size_of::<u32>())
        .send()?;

    // SAFETY: response payload is at least size_of::<u32>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) };
    Ok(out)
}

/// Attaches the join event (cmd 328).
pub(crate) fn attach_join_event(session: &Session) -> Result<u32, AttachEventError> {
    let result = session
        .dispatch(proto::ATTACH_JOIN_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send()
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
    let result = session
        .dispatch(proto::GET_MEMBERS)
        .out_size(size_of::<i32>())
        .buffer(
            members.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(members),
            BufferAttr::HIPC_AUTO_SELECT.or(BufferAttr::OUT),
        )
        .send()?;

    // SAFETY: response payload is at least size_of::<i32>().
    let total_out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) };
    Ok(total_out)
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

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
