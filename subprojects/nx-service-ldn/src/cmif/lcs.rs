//! `IUserLocalCommunicationService` / `ISystemLocalCommunicationService`
//! (sub-object of `ldn:u` / `ldn:s`) CMIF dispatch helpers.
//!
//! Mirrors libnx's `ldn*` free-function surface. The crate is hosversion
//! unaware — the caller is responsible for picking the right variant
//! (Initialize / `*_legacy` channel encoding / SetOperationMode cmd id) based
//! on HOS version.

use core::mem::size_of;

use nx_sf::{
    ipc::Handle as RawSessionHandle,
    service::{
        BufferAttr,
        DispatchError,
        DomainObjectRef,
        OutHandleAttr,
        OwnedSessionHandle,
    },
};

use crate::{
    dispatch::{
        dispatch_in,
        dispatch_no_io,
        dispatch_out,
    },
    proto::{
        CMD_LCS_402,
        CMD_LCS_403,
        CMD_LCS_ADD_ACCEPT_FILTER_ENTRY,
        CMD_LCS_CLEAR_ACCEPT_FILTER,
        CMD_LCS_CLOSE_ACCESS_POINT,
        CMD_LCS_CLOSE_STATION,
        CMD_LCS_CONNECT,
        CMD_LCS_CONNECT_PRIVATE,
        CMD_LCS_CREATE_NETWORK,
        CMD_LCS_CREATE_NETWORK_PRIVATE,
        CMD_LCS_DESTROY_NETWORK,
        CMD_LCS_DISABLE_ACTION_FRAME,
        CMD_LCS_DISCONNECT,
        CMD_LCS_ENABLE_ACTION_FRAME,
        CMD_LCS_FINALIZE,
        CMD_LCS_GET_DISCONNECT_REASON,
        CMD_LCS_GET_IPV4_ADDRESS,
        CMD_LCS_GET_NETWORK_CONFIG,
        CMD_LCS_GET_NETWORK_INFO,
        CMD_LCS_GET_NETWORK_INFO_AND_HISTORY,
        CMD_LCS_GET_SECURITY_PARAMETER,
        CMD_LCS_GET_STATE,
        CMD_LCS_GET_STATE_CHANGE_EVENT,
        CMD_LCS_INITIALIZE_LEGACY,
        CMD_LCS_INITIALIZE_WITH_PRIORITY,
        CMD_LCS_OPEN_ACCESS_POINT,
        CMD_LCS_OPEN_STATION,
        CMD_LCS_RECV_ACTION_FRAME,
        CMD_LCS_REJECT,
        CMD_LCS_RESET_TX_POWER,
        CMD_LCS_SCAN,
        CMD_LCS_SCAN_PRIVATE,
        CMD_LCS_SEND_ACTION_FRAME,
        CMD_LCS_SET_ADVERTISE_DATA,
        CMD_LCS_SET_HOME_CHANNEL,
        CMD_LCS_SET_PROTOCOL,
        CMD_LCS_SET_STATION_ACCEPT_POLICY,
        CMD_LCS_SET_TX_POWER,
        CMD_LCS_SET_WIRELESS_CONTROLLER_RESTRICTION,
        LdnAcceptPolicy,
        LdnDisconnectReason,
        LdnOperationMode,
        LdnProtocol,
        LdnScanFilterFlag,
        LdnServiceType,
        LdnState,
        LdnWirelessControllerRestriction,
    },
    types::{
        LdnActionFrameSettings,
        LdnAddressEntry,
        LdnIpv4Address,
        LdnMacAddress,
        LdnNetworkConfig,
        LdnNetworkInfo,
        LdnNodeLatestUpdate,
        LdnScanFilter,
        LdnSecurityConfig,
        LdnSecurityParameter,
        LdnSubnetMask,
        LdnUserConfig,
    },
};

/// `_ldnChannelToOldBand` — pre-20.0.0 ABI band selector.
#[inline]
pub fn channel_to_old_band(channel: i16) -> i16 {
    if channel < 15 { 24 } else { 50 }
}

/// `_ldnChannelToChannelBand` — `[20.0.0+]` band-packed-into-channel encoding.
#[inline]
pub fn channel_to_band(channel: i16) -> u16 {
    if channel == 0 {
        return 0;
    }
    let tmp_channel: u16 = (channel as u16) & 0x3FF;
    let band: u16 = if tmp_channel < 15 {
        2
    } else if (32..178).contains(&tmp_channel) {
        5
    } else {
        0x3F // Invalid
    };
    (channel as u16) | (band << 10)
}

/// `_ldnChannelBandToChannel` — strips the band bits from a packed value.
#[inline]
pub fn channel_band_to_channel(val: u16) -> i16 {
    (val & 0x3FF) as i16
}

// Initialize / Finalize / SetOperationMode (hosversion-aware variants exposed
// separately; the caller picks).

/// `Initialize` (cmd 400) — pre-`[7.0.0]` path. `send_pid` + zero payload.
pub(crate) fn initialize_legacy(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    let reserved: u64 = 0;
    // SAFETY: `reserved` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const reserved).cast::<u8>(), size_of::<u64>())
    };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(CMD_LCS_INITIALIZE_LEGACY)
        .send_pid()
        .in_raw(in_bytes)
        .send(&mut buf)
        .map(|_| ())
}

/// `InitializeWithVersion` — cmd 402 on `ldn:u` / cmd 403 on `ldn:s`.
pub(crate) fn initialize_with_version(
    object: DomainObjectRef<'_>,
    kind: LdnServiceType,
    version: i32,
) -> Result<(), DispatchError> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct In {
        version: i32,
        _pad: u32,
        _reserved: u64,
    }
    let cmd = match kind {
        LdnServiceType::User => CMD_LCS_402,
        LdnServiceType::System => CMD_LCS_403,
    };
    let input = In {
        version,
        _pad: 0,
        _reserved: 0,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<In>()) };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(cmd)
        .send_pid()
        .in_raw(in_bytes)
        .send(&mut buf)
        .map(|_| ())
}

/// `InitializeWithPriority` (cmd 404) — `ldn:s`-only, `[19.0.0+]`. The caller
/// must guard on hosversion + object kind.
pub(crate) fn initialize_with_priority(
    object: DomainObjectRef<'_>,
    version: i32,
    priority: i32,
) -> Result<(), DispatchError> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct In {
        version: i32,
        priority: i32,
        _reserved: u64,
    }
    let input = In {
        version,
        priority,
        _reserved: 0,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<In>()) };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(CMD_LCS_INITIALIZE_WITH_PRIORITY)
        .send_pid()
        .in_raw(in_bytes)
        .send(&mut buf)
        .map(|_| ())
}

/// `Finalize` (cmd 401).
pub(crate) fn finalize(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(&object, CMD_LCS_FINALIZE)
}

/// `SetOperationMode` — cmd 402 on `ldn:s` / cmd 403 on `ldn:u`. The cmd id is
/// the *opposite* of `InitializeWithVersion`'s (libnx parity).
pub(crate) fn set_operation_mode(
    object: DomainObjectRef<'_>,
    kind: LdnServiceType,
    mode: LdnOperationMode,
) -> Result<(), DispatchError> {
    let cmd = match kind {
        LdnServiceType::System => CMD_LCS_402,
        LdnServiceType::User => CMD_LCS_403,
    };
    dispatch_in(&object, cmd, mode as u32)
}

/// `GetState` (cmd 0).
pub(crate) fn get_state(object: DomainObjectRef<'_>) -> Result<LdnState, GetStateError> {
    let raw = dispatch_out::<u32>(&object, CMD_LCS_GET_STATE).map_err(GetStateError::Dispatch)?;
    LdnState::from_raw(raw).ok_or(GetStateError::InvalidState(raw))
}

#[derive(Debug, thiserror::Error)]
pub enum GetStateError {
    #[error("failed to dispatch GetState")]
    Dispatch(#[source] DispatchError),
    #[error("invalid LdnState: {0}")]
    InvalidState(u32),
}

/// `GetNetworkInfo` (cmd 1). Writes to a caller-supplied output buffer.
pub(crate) fn get_network_info(
    object: DomainObjectRef<'_>,
    out: &mut LdnNetworkInfo,
) -> Result<(), DispatchError> {
    // SAFETY: `out` is a valid `&mut LdnNetworkInfo`; viewing it as bytes for
    // the OUT buffer is sound, and the byte slice borrows `out`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            (out as *mut LdnNetworkInfo).cast::<u8>(),
            size_of::<LdnNetworkInfo>(),
        )
    };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(CMD_LCS_GET_NETWORK_INFO)
        .out_buffer(
            out_bytes,
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut buf)
        .map(|_| ())
}

/// `GetIpv4Address` (cmd 2).
pub(crate) fn get_ipv4_address(
    object: DomainObjectRef<'_>,
) -> Result<(LdnIpv4Address, LdnSubnetMask), DispatchError> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Out {
        addr: LdnIpv4Address,
        mask: LdnSubnetMask,
    }
    let out = dispatch_out::<Out>(&object, CMD_LCS_GET_IPV4_ADDRESS)?;
    Ok((out.addr, out.mask))
}

/// `GetDisconnectReason` (cmd 3).
pub(crate) fn get_disconnect_reason(
    object: DomainObjectRef<'_>,
) -> Result<LdnDisconnectReason, GetDisconnectReasonError> {
    let raw = dispatch_out::<i16>(&object, CMD_LCS_GET_DISCONNECT_REASON)
        .map_err(GetDisconnectReasonError::Dispatch)?;
    LdnDisconnectReason::from_raw(raw).ok_or(GetDisconnectReasonError::InvalidReason(raw))
}

#[derive(Debug, thiserror::Error)]
pub enum GetDisconnectReasonError {
    #[error("failed to dispatch GetDisconnectReason")]
    Dispatch(#[source] DispatchError),
    #[error("invalid LdnDisconnectReason: {0}")]
    InvalidReason(i16),
}

/// `GetSecurityParameter` (cmd 4).
pub(crate) fn get_security_parameter(
    object: DomainObjectRef<'_>,
) -> Result<LdnSecurityParameter, DispatchError> {
    dispatch_out::<LdnSecurityParameter>(&object, CMD_LCS_GET_SECURITY_PARAMETER)
}

/// `GetNetworkConfig` (cmd 5).
pub(crate) fn get_network_config(
    object: DomainObjectRef<'_>,
) -> Result<LdnNetworkConfig, DispatchError> {
    dispatch_out::<LdnNetworkConfig>(&object, CMD_LCS_GET_NETWORK_CONFIG)
}

/// `GetStateChangeEvent` (cmd 100). Returns the kernel handle of a *copy*
/// of the autoclear event; caller owns the handle and must close it.
pub(crate) fn get_state_change_event(
    object: DomainObjectRef<'_>,
) -> Result<OwnedSessionHandle, GetStateChangeEventError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(CMD_LCS_GET_STATE_CHANGE_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)
        .map_err(GetStateChangeEventError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(GetStateChangeEventError::MissingHandle);
    }
    // SAFETY: the kernel returned a valid handle in the copy-handle slot.
    Ok(OwnedSessionHandle::from_handle_unchecked(
        RawSessionHandle::from_raw_unchecked(result.copy_handles[0]),
    ))
}

#[derive(Debug, thiserror::Error)]
pub enum GetStateChangeEventError {
    #[error("failed to dispatch GetStateChangeEvent")]
    Dispatch(#[source] DispatchError),
    #[error("GetStateChangeEvent response did not include the event handle")]
    MissingHandle,
}

/// `GetNetworkInfoAndHistory` (cmd 101). Both buffers are HipcPointer/Out;
/// `network_info` is fixed-size, `nodes` is variable (always length 8 per
/// libnx but we pass the caller's slice).
pub(crate) fn get_network_info_and_history(
    object: DomainObjectRef<'_>,
    network_info: &mut LdnNetworkInfo,
    nodes: &mut [LdnNodeLatestUpdate],
) -> Result<(), DispatchError> {
    // SAFETY: `network_info` is a valid `&mut`; viewing it as bytes for OUT is sound.
    let net_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            (network_info as *mut LdnNetworkInfo).cast::<u8>(),
            size_of::<LdnNetworkInfo>(),
        )
    };
    // SAFETY: `nodes` is a valid `&mut` slice; viewing it as bytes for OUT is sound.
    let nodes_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            nodes.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(nodes),
        )
    };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(CMD_LCS_GET_NETWORK_INFO_AND_HISTORY)
        .out_buffer(
            net_bytes,
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .out_buffer(nodes_bytes, BufferAttr::HIPC_POINTER)
        .send(&mut buf)
        .map(|_| ())
}

/// `Scan` (cmd 102). The flag mask matches libnx's `ldnScan` (`flags & 0x37`).
/// Returns the number of network entries the server actually wrote.
pub(crate) fn scan(
    object: DomainObjectRef<'_>,
    channel: i16,
    filter: &LdnScanFilter,
    out_buf: &mut [LdnNetworkInfo],
) -> Result<i32, DispatchError> {
    let mut tmp_filter = *filter;
    tmp_filter.flags = LdnScanFilterFlag(tmp_filter.flags.bits() & 0x37);
    scan_inner(object, CMD_LCS_SCAN, channel, &tmp_filter, out_buf)
}

/// `ScanPrivate` (cmd 103). `flags & 0x3F`.
pub(crate) fn scan_private(
    object: DomainObjectRef<'_>,
    channel: i16,
    filter: &LdnScanFilter,
    out_buf: &mut [LdnNetworkInfo],
) -> Result<i32, DispatchError> {
    let mut tmp_filter = *filter;
    tmp_filter.flags = LdnScanFilterFlag(tmp_filter.flags.bits() & 0x3F);
    scan_inner(object, CMD_LCS_SCAN_PRIVATE, channel, &tmp_filter, out_buf)
}

fn scan_inner(
    object: DomainObjectRef<'_>,
    cmd: u32,
    channel: i16,
    filter: &LdnScanFilter,
    out_buf: &mut [LdnNetworkInfo],
) -> Result<i32, DispatchError> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct In {
        channel: i16,
        pad: [u8; 6],
        filter: LdnScanFilter,
    }
    let input = In {
        channel,
        pad: [0; 6],
        filter: *filter,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<In>()) };
    // SAFETY: `out_buf` is a valid `&mut` slice; viewing it as bytes for the
    // OUT buffer is sound.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            out_buf.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(out_buf),
        )
    };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(cmd)
        .in_raw(in_bytes)
        .out_size(size_of::<i16>())
        .out_buffer(out_bytes, BufferAttr::HIPC_AUTO_SELECT)
        .send(&mut buf)?;

    let total: i16 = if result.data.len() >= 2 {
        i16::from_le_bytes([result.data[0], result.data[1]])
    } else {
        0
    };
    Ok(total as i32)
}

/// `SetWirelessControllerRestriction` (cmd 104).
pub(crate) fn set_wireless_controller_restriction(
    object: DomainObjectRef<'_>,
    restriction: LdnWirelessControllerRestriction,
) -> Result<(), DispatchError> {
    dispatch_in(
        &object,
        CMD_LCS_SET_WIRELESS_CONTROLLER_RESTRICTION,
        restriction as u32,
    )
}

/// `SetProtocol` (cmd 106, `[18.0.0+]`).
pub(crate) fn set_protocol(
    object: DomainObjectRef<'_>,
    protocol: LdnProtocol,
) -> Result<(), DispatchError> {
    dispatch_in(&object, CMD_LCS_SET_PROTOCOL, protocol as u32)
}

/// `OpenAccessPoint` (cmd 200).
pub(crate) fn open_access_point(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(&object, CMD_LCS_OPEN_ACCESS_POINT)
}

/// `CloseAccessPoint` (cmd 201).
pub(crate) fn close_access_point(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(&object, CMD_LCS_CLOSE_ACCESS_POINT)
}

/// `DestroyNetwork` (cmd 204).
pub(crate) fn destroy_network(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(&object, CMD_LCS_DESTROY_NETWORK)
}

/// `Reject` (cmd 205).
pub(crate) fn reject(
    object: DomainObjectRef<'_>,
    addr: LdnIpv4Address,
) -> Result<(), DispatchError> {
    dispatch_in(&object, CMD_LCS_REJECT, addr)
}

/// `SetStationAcceptPolicy` (cmd 207).
pub(crate) fn set_station_accept_policy(
    object: DomainObjectRef<'_>,
    policy: LdnAcceptPolicy,
) -> Result<(), DispatchError> {
    dispatch_in::<u8>(&object, CMD_LCS_SET_STATION_ACCEPT_POLICY, policy as u8)
}

/// `AddAcceptFilterEntry` (cmd 208).
pub(crate) fn add_accept_filter_entry(
    object: DomainObjectRef<'_>,
    addr: LdnMacAddress,
) -> Result<(), DispatchError> {
    dispatch_in(&object, CMD_LCS_ADD_ACCEPT_FILTER_ENTRY, addr)
}

/// `ClearAcceptFilter` (cmd 209).
pub(crate) fn clear_accept_filter(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(&object, CMD_LCS_CLEAR_ACCEPT_FILTER)
}

/// `OpenStation` (cmd 300).
pub(crate) fn open_station(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(&object, CMD_LCS_OPEN_STATION)
}

/// `CloseStation` (cmd 301).
pub(crate) fn close_station(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(&object, CMD_LCS_CLOSE_STATION)
}

/// `Disconnect` (cmd 304).
pub(crate) fn disconnect(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(&object, CMD_LCS_DISCONNECT)
}

/// `SetAdvertiseData` (cmd 206). `data == &[]` resets the AdvertiseData.
pub(crate) fn set_advertise_data(
    object: DomainObjectRef<'_>,
    data: &[u8],
) -> Result<(), DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(CMD_LCS_SET_ADVERTISE_DATA)
        .in_buffer(data, BufferAttr::HIPC_AUTO_SELECT)
        .send(&mut buf)
        .map(|_| ())
}

// Network create / connect (the big payloads).

fn sanitize_user_config(src: &LdnUserConfig) -> LdnUserConfig {
    let mut out = LdnUserConfig {
        user_name: [0; 0x21],
        reserved: [0; 0xF],
    };
    // libnx copies `sizeof(user_name)-1` bytes, leaving the trailing NUL.
    out.user_name[..0x20].copy_from_slice(&src.user_name[..0x20]);
    out
}

fn sanitize_network_config(src: &LdnNetworkConfig) -> LdnNetworkConfig {
    // libnx zeroes the temp struct and copies just five fields.
    let mut out = LdnNetworkConfig {
        intent_id: crate::types::LdnIntentId {
            local_communication_id: 0,
            reserved_x8: [0; 2],
            scene_id: 0,
            reserved_xc: [0; 4],
        },
        channel: 0,
        node_count_max: 0,
        reserved_x13: 0,
        local_communication_version: 0,
        reserved_x16: [0; 0xA],
    };
    out.intent_id.local_communication_id = src.intent_id.local_communication_id;
    out.intent_id.scene_id = src.intent_id.scene_id;
    out.channel = src.channel;
    out.node_count_max = src.node_count_max;
    out.local_communication_version = src.local_communication_version;
    out
}

/// `CreateNetwork` (cmd 202).
pub(crate) fn create_network(
    object: DomainObjectRef<'_>,
    sec_config: &LdnSecurityConfig,
    user_config: &LdnUserConfig,
    network_config: &LdnNetworkConfig,
) -> Result<(), DispatchError> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct In {
        sec_config: LdnSecurityConfig,
        user_config: LdnUserConfig,
        _pad: u32,
        network_config: LdnNetworkConfig,
    }
    let input = In {
        sec_config: *sec_config,
        user_config: sanitize_user_config(user_config),
        _pad: 0,
        network_config: sanitize_network_config(network_config),
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<In>()) };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(CMD_LCS_CREATE_NETWORK)
        .in_raw(in_bytes)
        .send(&mut buf)
        .map(|_| ())
}

/// `CreateNetworkPrivate` (cmd 203). `addrs == &[]` yields a non-private
/// network (libnx parity).
pub(crate) fn create_network_private(
    object: DomainObjectRef<'_>,
    sec_config: &LdnSecurityConfig,
    sec_param: &LdnSecurityParameter,
    user_config: &LdnUserConfig,
    network_config: &LdnNetworkConfig,
    addrs: &[LdnAddressEntry],
) -> Result<(), DispatchError> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct In {
        sec_config: LdnSecurityConfig,
        sec_param: LdnSecurityParameter,
        user_config: LdnUserConfig,
        _pad: u32,
        network_config: LdnNetworkConfig,
    }
    let input = In {
        sec_config: *sec_config,
        sec_param: *sec_param,
        user_config: sanitize_user_config(user_config),
        _pad: 0,
        network_config: sanitize_network_config(network_config),
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<In>()) };
    // SAFETY: `addrs` is a valid `&[LdnAddressEntry]`; viewing it as bytes for
    // the IN buffer is sound.
    let addr_bytes = unsafe {
        core::slice::from_raw_parts(addrs.as_ptr().cast::<u8>(), core::mem::size_of_val(addrs))
    };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(CMD_LCS_CREATE_NETWORK_PRIVATE)
        .in_raw(in_bytes)
        .in_buffer(addr_bytes, BufferAttr::HIPC_POINTER)
        .send(&mut buf)
        .map(|_| ())
}

/// `Connect` (cmd 302).
pub(crate) fn connect(
    object: DomainObjectRef<'_>,
    sec_config: &LdnSecurityConfig,
    user_config: &LdnUserConfig,
    version: i32,
    option: u32,
    network_info: &LdnNetworkInfo,
) -> Result<(), DispatchError> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct In {
        sec_config: LdnSecurityConfig,
        user_config: LdnUserConfig,
        version: i32,
        option: u32,
    }
    let input = In {
        sec_config: *sec_config,
        user_config: sanitize_user_config(user_config),
        version,
        option,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<In>()) };
    // SAFETY: `network_info` is a valid `&LdnNetworkInfo`; viewing it as bytes
    // for the IN buffer is sound.
    let net_bytes = unsafe {
        core::slice::from_raw_parts(
            (network_info as *const LdnNetworkInfo).cast::<u8>(),
            size_of::<LdnNetworkInfo>(),
        )
    };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(CMD_LCS_CONNECT)
        .in_raw(in_bytes)
        .in_buffer(
            net_bytes,
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut buf)
        .map(|_| ())
}

/// `ConnectPrivate` (cmd 303).
pub(crate) fn connect_private(
    object: DomainObjectRef<'_>,
    sec_config: &LdnSecurityConfig,
    sec_param: &LdnSecurityParameter,
    user_config: &LdnUserConfig,
    version: i32,
    option: u32,
    network_config: &LdnNetworkConfig,
) -> Result<(), DispatchError> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct In {
        sec_config: LdnSecurityConfig,
        sec_param: LdnSecurityParameter,
        user_config: LdnUserConfig,
        version: i32,
        option: u32,
        _pad: u32,
        network_config: LdnNetworkConfig,
    }
    let input = In {
        sec_config: *sec_config,
        sec_param: *sec_param,
        user_config: sanitize_user_config(user_config),
        version,
        option,
        _pad: 0,
        network_config: sanitize_network_config(network_config),
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<In>()) };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(CMD_LCS_CONNECT_PRIVATE)
        .in_raw(in_bytes)
        .send(&mut buf)
        .map(|_| ())
}

/// `EnableActionFrame` (cmd 500, `[18.0.0+]`).
pub(crate) fn enable_action_frame(
    object: DomainObjectRef<'_>,
    settings: &LdnActionFrameSettings,
) -> Result<(), DispatchError> {
    dispatch_in(&object, CMD_LCS_ENABLE_ACTION_FRAME, *settings)
}

/// `DisableActionFrame` (cmd 501, `[18.0.0+]`).
pub(crate) fn disable_action_frame(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(&object, CMD_LCS_DISABLE_ACTION_FRAME)
}

/// `SendActionFrame` (cmd 502, `[18.0.0+]`) — pre-20.0.0 ABI: separate
/// `band` + `channel` fields.
pub(crate) fn send_action_frame_legacy(
    object: DomainObjectRef<'_>,
    data: &[u8],
    destination: LdnMacAddress,
    bssid: LdnMacAddress,
    channel: i16,
    flags: u32,
) -> Result<(), DispatchError> {
    send_action_frame_inner(
        object,
        data,
        destination,
        bssid,
        channel_to_old_band(channel),
        channel,
        flags,
    )
}

/// `SendActionFrame` (cmd 502, `[20.0.0+]`) — band packed into the `band` slot
/// using [`channel_to_band`]; `channel` field is unused.
pub(crate) fn send_action_frame(
    object: DomainObjectRef<'_>,
    data: &[u8],
    destination: LdnMacAddress,
    bssid: LdnMacAddress,
    channel: i16,
    flags: u32,
) -> Result<(), DispatchError> {
    send_action_frame_inner(
        object,
        data,
        destination,
        bssid,
        channel_to_band(channel) as i16,
        0,
        flags,
    )
}

fn send_action_frame_inner(
    object: DomainObjectRef<'_>,
    data: &[u8],
    destination: LdnMacAddress,
    bssid: LdnMacAddress,
    band: i16,
    channel: i16,
    flags: u32,
) -> Result<(), DispatchError> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct In {
        destination: LdnMacAddress,
        bssid: LdnMacAddress,
        band: i16,
        channel: i16,
        flags: u32,
    }
    let input = In {
        destination,
        bssid,
        band,
        channel,
        flags,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<In>()) };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(CMD_LCS_SEND_ACTION_FRAME)
        .in_raw(in_bytes)
        .in_buffer(data, BufferAttr::HIPC_AUTO_SELECT)
        .send(&mut buf)
        .map(|_| ())
}

/// Decoded output of `RecvActionFrame`.
#[derive(Debug, Clone, Copy)]
pub struct RecvActionFrameOut {
    pub addr0: LdnMacAddress,
    pub addr1: LdnMacAddress,
    pub channel: i16,
    pub size: u32,
    pub link_level: i32,
}

/// `RecvActionFrame` (cmd 503, `[18.0.0+]`) — pre-20.0.0 ABI: `channel` is
/// the raw response field, `band` is discarded.
pub(crate) fn recv_action_frame_legacy(
    object: DomainObjectRef<'_>,
    data: &mut [u8],
    flags: u32,
) -> Result<RecvActionFrameOut, DispatchError> {
    let raw = recv_action_frame_inner(object, data, flags)?;
    Ok(RecvActionFrameOut {
        addr0: raw.addr0,
        addr1: raw.addr1,
        channel: raw.channel,
        size: raw.size,
        link_level: raw.link_level,
    })
}

/// `RecvActionFrame` (cmd 503, `[20.0.0+]`) — `channel` is decoded from the
/// packed `band` slot via [`channel_band_to_channel`].
pub(crate) fn recv_action_frame(
    object: DomainObjectRef<'_>,
    data: &mut [u8],
    flags: u32,
) -> Result<RecvActionFrameOut, DispatchError> {
    let raw = recv_action_frame_inner(object, data, flags)?;
    Ok(RecvActionFrameOut {
        addr0: raw.addr0,
        addr1: raw.addr1,
        channel: channel_band_to_channel(raw.band as u16),
        size: raw.size,
        link_level: raw.link_level,
    })
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RecvActionFrameRaw {
    addr0: LdnMacAddress,
    addr1: LdnMacAddress,
    band: i16,
    channel: i16,
    size: u32,
    link_level: i32,
}

fn recv_action_frame_inner(
    object: DomainObjectRef<'_>,
    data: &mut [u8],
    flags: u32,
) -> Result<RecvActionFrameRaw, DispatchError> {
    // SAFETY: `flags` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const flags).cast::<u8>(), size_of::<u32>()) };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(CMD_LCS_RECV_ACTION_FRAME)
        .in_raw(in_bytes)
        .out_size(size_of::<RecvActionFrameRaw>())
        .out_buffer(data, BufferAttr::HIPC_AUTO_SELECT)
        .send(&mut buf)?;
    // SAFETY: response payload is at least size_of::<RecvActionFrameRaw>() by
    // virtue of `out_size`; parse_response would have errored otherwise.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<RecvActionFrameRaw>()) })
}

/// `SetHomeChannel` (cmd 505, `[18.0.0+]`) — pre-20.0.0 ABI: `{band, channel}`
/// payload.
pub(crate) fn set_home_channel_legacy(
    object: DomainObjectRef<'_>,
    channel: i16,
) -> Result<(), DispatchError> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct In {
        band: i16,
        channel: i16,
    }
    let input = In {
        band: channel_to_old_band(channel),
        channel,
    };
    dispatch_in(&object, CMD_LCS_SET_HOME_CHANNEL, input)
}

/// `SetHomeChannel` (cmd 505, `[20.0.0+]`) — band-packed `u16` payload.
pub(crate) fn set_home_channel(
    object: DomainObjectRef<'_>,
    channel: i16,
) -> Result<(), DispatchError> {
    dispatch_in::<u16>(&object, CMD_LCS_SET_HOME_CHANNEL, channel_to_band(channel))
}

/// `SetTxPower` (cmd 600, `[18.0.0+]`).
pub(crate) fn set_tx_power(object: DomainObjectRef<'_>, power: i16) -> Result<(), DispatchError> {
    dispatch_in::<u16>(&object, CMD_LCS_SET_TX_POWER, power as u16)
}

/// `ResetTxPower` (cmd 601, `[18.0.0+]`).
pub(crate) fn reset_tx_power(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(&object, CMD_LCS_RESET_TX_POWER)
}
