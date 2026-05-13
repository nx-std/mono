//! `IGeneralService` (sub-object of `nifm:u`/`nifm:s`/`nifm:a`) CMIF dispatch helpers.
//!
//! Mirrors libnx's free-function surface in `nifm.c`. The crate is hosversion
//! unaware — the caller is responsible for skipping
//! [`set_wowl_delayed_wake_time`] on pre-`[9.0.0]` firmware and for picking
//! `Admin`/`System` for [`set_wireless_communication_enabled`].

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, DomainObject};

use crate::{
    dispatch::{dispatch_in, dispatch_no_io, dispatch_out},
    proto::{
        CMD_IGS_CREATE_REQUEST, CMD_IGS_ENUMERATE_NETWORK_PROFILES, CMD_IGS_GET_CLIENT_ID,
        CMD_IGS_GET_CURRENT_IP_ADDRESS, CMD_IGS_GET_CURRENT_IP_CONFIG_INFO,
        CMD_IGS_GET_CURRENT_NETWORK_PROFILE, CMD_IGS_GET_INTERNET_CONNECTION_STATUS,
        CMD_IGS_GET_NETWORK_PROFILE, CMD_IGS_IS_ANY_FOREGROUND_REQUEST_ACCEPTED,
        CMD_IGS_IS_ANY_INTERNET_REQUEST_ACCEPTED, CMD_IGS_IS_ETHERNET_COMMUNICATION_ENABLED,
        CMD_IGS_IS_WIRELESS_COMMUNICATION_ENABLED, CMD_IGS_PUT_TO_SLEEP,
        CMD_IGS_SET_NETWORK_PROFILE, CMD_IGS_SET_WIRELESS_COMMUNICATION_ENABLED,
        CMD_IGS_SET_WOWL_DELAYED_WAKE_TIME, CMD_IGS_WAKE_UP, NifmInternetConnectionStatus,
        NifmInternetConnectionType, NifmNetworkProfileType,
    },
    types::{
        InternetConnection, IpConfigInfo, NifmClientId, NifmIpAddressSetting, NifmIpV4Address,
        NifmNetworkProfileBasicInfo, NifmNetworkProfileData, NifmSfNetworkProfileBasicInfo,
        NifmSfNetworkProfileData, Uuid, sf_from_network_profile_data,
        sf_to_network_profile_basic_info, sf_to_network_profile_data,
    },
};

//
// GetClientId (cmd 1).
//

/// `GetClientId` (cmd 1). libnx returns `id = 0` on failure; we surface the
/// dispatch error instead.
pub(crate) fn get_client_id(object: &DomainObject<'_>) -> Result<NifmClientId, DispatchError> {
    let mut out = NifmClientId::default();
    object
        .dispatch(CMD_IGS_GET_CLIENT_ID)
        .buffer(
            (&raw mut out).cast::<u8>(),
            size_of::<NifmClientId>(),
            BufferAttr::OUT
                .or(BufferAttr::HIPC_POINTER)
                .or(BufferAttr::FIXED_SIZE),
        )
        .send()
        .map(|_| out)
}

//
// CreateRequest (cmd 4).
//

/// `CreateRequest` (cmd 4). Server-side `s32 = 0x2` selector; returns the
/// newly-allocated domain sub-object id for the `IRequest`. The freshly
/// minted `DomainObject` is kept alive via [`ManuallyDrop`] so the pool can
/// re-open it per request.
pub(crate) fn create_request(object: &DomainObject<'_>) -> Result<u32, CreateRequestError> {
    let selector: i32 = 0x2;
    // SAFETY: `selector` lives on the stack until `send()` returns.
    let mut result = unsafe {
        object
            .dispatch(CMD_IGS_CREATE_REQUEST)
            .in_raw((&raw const selector).cast::<u8>(), size_of::<i32>())
            .out_objects(1)
            .send()
            .map_err(CreateRequestError::Dispatch)?
    };

    let new_object = result
        .take_object(0)
        .ok_or(CreateRequestError::MissingObject)?;
    Ok(core::mem::ManuallyDrop::new(new_object)
        .object_id()
        .to_raw())
}

/// Error returned by [`create_request`].
#[derive(Debug, thiserror::Error)]
pub enum CreateRequestError {
    /// IPC dispatch failed.
    #[error("failed to dispatch CreateRequest")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected sub-object id.
    #[error("CreateRequest response did not include the expected sub-object")]
    MissingObject,
}

//
// GetCurrentNetworkProfile (cmd 5).
//

/// `GetCurrentNetworkProfile` (cmd 5). Writes to `out` after converting from the
/// wire-side `Sf` layout.
pub(crate) fn get_current_network_profile(
    object: &DomainObject<'_>,
    out: &mut NifmNetworkProfileData,
) -> Result<(), DispatchError> {
    let mut sf: NifmSfNetworkProfileData = unsafe { core::mem::zeroed() };
    object
        .dispatch(CMD_IGS_GET_CURRENT_NETWORK_PROFILE)
        .buffer(
            (&raw mut sf).cast::<u8>(),
            size_of::<NifmSfNetworkProfileData>(),
            BufferAttr::OUT
                .or(BufferAttr::HIPC_POINTER)
                .or(BufferAttr::FIXED_SIZE),
        )
        .send()?;
    sf_to_network_profile_data(&sf, out);
    Ok(())
}

//
// EnumerateNetworkProfiles (cmd 7).
//

/// `EnumerateNetworkProfiles` (cmd 7). Returns the total number of profiles the
/// server reports; the caller's `buffer` is filled with up to `buffer.len()`
/// entries, each converted from the wire layout in place.
pub(crate) fn enumerate_network_profiles(
    object: &DomainObject<'_>,
    kind: NifmNetworkProfileType,
    buffer: &mut [NifmNetworkProfileBasicInfo],
) -> Result<i32, DispatchError> {
    let in_kind: u8 = kind.as_raw();
    // SAFETY: `in_kind` lives on the stack until `send()` returns. The buffer
    // is exposed to the kernel via HipcMapAlias; the slice's lifetime covers
    // the duration of the call.
    let result = unsafe {
        object
            .dispatch(CMD_IGS_ENUMERATE_NETWORK_PROFILES)
            .in_raw((&raw const in_kind).cast::<u8>(), size_of::<u8>())
            .out_size(size_of::<i32>())
            .buffer(
                buffer.as_mut_ptr().cast::<u8>(),
                size_of::<NifmSfNetworkProfileBasicInfo>() * buffer.len(),
                BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()?
    };

    let total_entries = i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]);

    let max = buffer.len() as i32;
    let returned = if total_entries < max {
        total_entries
    } else {
        max
    };
    // Convert entries in place from the wire layout (`Sf`) into the app layout.
    // libnx walks backwards so the conversion does not overwrite later
    // wire-side entries before they are read; we do the same.
    for i in (0..returned).rev() {
        let idx = i as usize;
        // SAFETY: at this point `buffer[idx]` still holds the wire-side bytes
        // (`NifmSfNetworkProfileBasicInfo`, 0x75 bytes), which fit within the
        // app-side slot (`NifmNetworkProfileBasicInfo`, 0x78 bytes). We read
        // the wire-side struct out and then overwrite the slot with the
        // converted app-side struct.
        let sf: NifmSfNetworkProfileBasicInfo = unsafe {
            core::ptr::read_unaligned(
                buffer
                    .as_ptr()
                    .add(idx)
                    .cast::<NifmSfNetworkProfileBasicInfo>(),
            )
        };
        sf_to_network_profile_basic_info(&sf, &mut buffer[idx]);
    }
    Ok(total_entries)
}

//
// GetNetworkProfile (cmd 8).
//

/// `GetNetworkProfile` (cmd 8). UUID in by value, profile out via HipcPointer.
pub(crate) fn get_network_profile(
    object: &DomainObject<'_>,
    uuid: Uuid,
    out: &mut NifmNetworkProfileData,
) -> Result<(), DispatchError> {
    let mut sf: NifmSfNetworkProfileData = unsafe { core::mem::zeroed() };
    // SAFETY: `uuid` lives on the stack until `send()` returns.
    unsafe {
        object
            .dispatch(CMD_IGS_GET_NETWORK_PROFILE)
            .in_raw((&raw const uuid).cast::<u8>(), size_of::<Uuid>())
            .buffer(
                (&raw mut sf).cast::<u8>(),
                size_of::<NifmSfNetworkProfileData>(),
                BufferAttr::OUT
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::FIXED_SIZE),
            )
            .send()?;
    }
    sf_to_network_profile_data(&sf, out);
    Ok(())
}

//
// SetNetworkProfile (cmd 9).
//

/// `SetNetworkProfile` (cmd 9). App-layout profile in, UUID out by value.
/// Only available with `Admin`; the caller decides whether to invoke.
pub(crate) fn set_network_profile(
    object: &DomainObject<'_>,
    profile: &NifmNetworkProfileData,
) -> Result<Uuid, DispatchError> {
    let mut sf: NifmSfNetworkProfileData = unsafe { core::mem::zeroed() };
    sf_from_network_profile_data(profile, &mut sf);
    let result = object
        .dispatch(CMD_IGS_SET_NETWORK_PROFILE)
        .buffer(
            (&raw const sf).cast::<u8>(),
            size_of::<NifmSfNetworkProfileData>(),
            BufferAttr::IN
                .or(BufferAttr::HIPC_POINTER)
                .or(BufferAttr::FIXED_SIZE),
        )
        .out_size(size_of::<Uuid>())
        .send()?;

    // SAFETY: response payload is at least size_of::<Uuid>() bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<Uuid>()) })
}

//
// GetCurrentIpAddress (cmd 12).
//

/// `GetCurrentIpAddress` (cmd 12). Returns the IPv4 address as a 4-byte payload.
pub(crate) fn get_current_ip_address(
    object: &DomainObject<'_>,
) -> Result<NifmIpV4Address, DispatchError> {
    dispatch_out::<NifmIpV4Address>(object, CMD_IGS_GET_CURRENT_IP_ADDRESS)
}

//
// GetCurrentIpConfigInfo (cmd 15).
//

/// `GetCurrentIpConfigInfo` (cmd 15). libnx fetches a packed
/// `(IpAddressSetting + DnsSetting)` payload and splits it into five `u32*`
/// out-parameters; we surface the parsed view as [`IpConfigInfo`].
pub(crate) fn get_current_ip_config_info(
    object: &DomainObject<'_>,
) -> Result<IpConfigInfo, DispatchError> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Out {
        ip_setting: NifmIpAddressSetting,
        dns_setting: crate::types::NifmDnsSetting,
    }
    let raw = dispatch_out::<Out>(object, CMD_IGS_GET_CURRENT_IP_CONFIG_INFO)?;
    Ok(IpConfigInfo {
        current_addr: raw.ip_setting.current_addr,
        subnet_mask: raw.ip_setting.subnet_mask,
        gateway: raw.ip_setting.gateway,
        primary_dns_server: raw.dns_setting.primary_dns_server,
        secondary_dns_server: raw.dns_setting.secondary_dns_server,
    })
}

//
// SetWirelessCommunicationEnabled (cmd 16).
//

/// `SetWirelessCommunicationEnabled` (cmd 16). libnx restricts this to
/// `System` / `Admin`; the caller-facing wrapper enforces that, this helper
/// dispatches unconditionally.
pub(crate) fn set_wireless_communication_enabled(
    object: &DomainObject<'_>,
    enable: bool,
) -> Result<(), DispatchError> {
    let raw: u8 = if enable { 1 } else { 0 };
    dispatch_in(object, CMD_IGS_SET_WIRELESS_COMMUNICATION_ENABLED, raw)
}

//
// IsWirelessCommunicationEnabled (cmd 17).
//

/// `IsWirelessCommunicationEnabled` (cmd 17).
pub(crate) fn is_wireless_communication_enabled(
    object: &DomainObject<'_>,
) -> Result<bool, DispatchError> {
    let raw = dispatch_out::<u8>(object, CMD_IGS_IS_WIRELESS_COMMUNICATION_ENABLED)?;
    Ok((raw & 1) != 0)
}

//
// GetInternetConnectionStatus (cmd 18).
//

/// `GetInternetConnectionStatus` (cmd 18).
pub(crate) fn get_internet_connection_status(
    object: &DomainObject<'_>,
) -> Result<InternetConnection, GetInternetConnectionStatusError> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Out {
        connection_type: u8,
        wifi_strength: u8,
        status: u8,
    }
    let out = dispatch_out::<Out>(object, CMD_IGS_GET_INTERNET_CONNECTION_STATUS)
        .map_err(GetInternetConnectionStatusError::Dispatch)?;

    let connection_type = NifmInternetConnectionType::from_raw(out.connection_type).ok_or(
        GetInternetConnectionStatusError::InvalidConnectionType(out.connection_type),
    )?;
    let status = NifmInternetConnectionStatus::from_raw(out.status)
        .ok_or(GetInternetConnectionStatusError::InvalidStatus(out.status))?;
    Ok(InternetConnection {
        connection_type,
        wifi_strength: out.wifi_strength,
        status,
    })
}

/// Error returned by [`get_internet_connection_status`].
#[derive(Debug, thiserror::Error)]
pub enum GetInternetConnectionStatusError {
    /// CMIF dispatch failed.
    #[error("failed to dispatch GetInternetConnectionStatus")]
    Dispatch(#[source] DispatchError),
    /// Service returned a connection-type value outside the documented range.
    #[error("invalid NifmInternetConnectionType: {0}")]
    InvalidConnectionType(u8),
    /// Service returned a status value outside the documented range.
    #[error("invalid NifmInternetConnectionStatus: {0}")]
    InvalidStatus(u8),
}

//
// IsEthernetCommunicationEnabled (cmd 20).
//

/// `IsEthernetCommunicationEnabled` (cmd 20).
pub(crate) fn is_ethernet_communication_enabled(
    object: &DomainObject<'_>,
) -> Result<bool, DispatchError> {
    let raw = dispatch_out::<u8>(object, CMD_IGS_IS_ETHERNET_COMMUNICATION_ENABLED)?;
    Ok((raw & 1) != 0)
}

//
// IsAnyInternetRequestAccepted (cmd 21).
//

/// `IsAnyInternetRequestAccepted` (cmd 21). libnx returns `false` on dispatch
/// failure; we surface the dispatch error instead.
pub(crate) fn is_any_internet_request_accepted(
    object: &DomainObject<'_>,
    id: NifmClientId,
) -> Result<bool, DispatchError> {
    let result = object
        .dispatch(CMD_IGS_IS_ANY_INTERNET_REQUEST_ACCEPTED)
        .buffer(
            (&raw const id).cast::<u8>(),
            size_of::<NifmClientId>(),
            BufferAttr::IN
                .or(BufferAttr::HIPC_POINTER)
                .or(BufferAttr::FIXED_SIZE),
        )
        .out_size(size_of::<u8>())
        .send()?;
    Ok((result.data[0] & 1) != 0)
}

//
// IsAnyForegroundRequestAccepted (cmd 22).
//

/// `IsAnyForegroundRequestAccepted` (cmd 22).
pub(crate) fn is_any_foreground_request_accepted(
    object: &DomainObject<'_>,
) -> Result<bool, DispatchError> {
    let raw = dispatch_out::<u8>(object, CMD_IGS_IS_ANY_FOREGROUND_REQUEST_ACCEPTED)?;
    Ok((raw & 1) != 0)
}

//
// PutToSleep / WakeUp (cmds 23, 24).
//

/// `PutToSleep` (cmd 23).
pub(crate) fn put_to_sleep(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(object, CMD_IGS_PUT_TO_SLEEP)
}

/// `WakeUp` (cmd 24).
pub(crate) fn wake_up(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(object, CMD_IGS_WAKE_UP)
}

//
// SetWowlDelayedWakeTime (cmd 43, [9.0.0+]).
//

/// `SetWowlDelayedWakeTime` (cmd 43). Caller must guard on `[9.0.0+]`.
pub(crate) fn set_wowl_delayed_wake_time(
    object: &DomainObject<'_>,
    val: i32,
) -> Result<(), DispatchError> {
    dispatch_in(object, CMD_IGS_SET_WOWL_DELAYED_WAKE_TIME, val as u32)
}
