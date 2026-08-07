//! Wire-format types for the `nifm` service.
//!
//! Every `#[repr(C)]` struct here mirrors the layout used by libnx's
//! `nx/include/switch/services/nifm.h` so that IPC buffers can be cast in
//! place. Sizes and offsets are pinned with `static_assertions::const_assert_eq!`.

use core::mem::{
    offset_of,
    size_of,
};

use static_assertions::const_assert_eq;
use zerocopy::FromZeros as _;

/// 128-bit UUID — wire-equivalent to libnx's `typedef struct { u8 uuid[0x10]; } Uuid;`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct Uuid {
    pub bytes: [u8; 16],
}

const_assert_eq!(size_of::<Uuid>(), 0x10);

/// IPv4 address as a 4-byte big-endian payload (`struct in_addr`).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NifmIpV4Address {
    pub addr: [u8; 4],
}

const_assert_eq!(size_of::<NifmIpV4Address>(), 0x4);

impl NifmIpV4Address {
    /// Reinterprets the 4-byte payload as a raw `u32` (host endian), matching
    /// libnx's `*((u32*)addr.addr)` cast.
    #[inline]
    pub const fn as_u32(self) -> u32 {
        u32::from_ne_bytes(self.addr)
    }
}

/// IPv4 settings (current address + subnet + gateway).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NifmIpAddressSetting {
    pub is_automatic: u8,
    pub current_addr: NifmIpV4Address,
    pub subnet_mask: NifmIpV4Address,
    pub gateway: NifmIpV4Address,
}

const_assert_eq!(size_of::<NifmIpAddressSetting>(), 0xD);

/// DNS settings (primary + secondary).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NifmDnsSetting {
    pub is_automatic: u8,
    pub primary_dns_server: NifmIpV4Address,
    pub secondary_dns_server: NifmIpV4Address,
}

const_assert_eq!(size_of::<NifmDnsSetting>(), 0x9);

/// HTTP proxy settings.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NifmProxySetting {
    pub enabled: u8,
    pub pad: u8,
    pub port: u16,
    pub server: [u8; 0x64],
    pub auto_auth_enabled: u8,
    pub user: [u8; 0x20],
    pub password: [u8; 0x20],
    pub pad2: u8,
}

const_assert_eq!(size_of::<NifmProxySetting>(), 0xAA);

/// Full IP settings bundle.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NifmIpSettingData {
    pub ip_address_setting: NifmIpAddressSetting,
    pub dns_setting: NifmDnsSetting,
    pub proxy_setting: NifmProxySetting,
    pub mtu: u16,
}

// 0x0D + 0x09 + 0xAA + 0x02 = 0xC2.
const_assert_eq!(size_of::<NifmIpSettingData>(), 0xC2);

/// Wire-side wireless-settings payload as serialized by `IGeneralService`.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NifmSfWirelessSettingData {
    pub ssid_len: u8,
    pub ssid: [u8; 0x20],
    pub unk_x21: u8,
    pub unk_x22: u8,
    pub unk_x23: u8,
    pub passphrase: [u8; 0x41],
}

const_assert_eq!(size_of::<NifmSfWirelessSettingData>(), 0x65);

/// Application-side wireless-settings payload as exposed by libnx's
/// `NifmNetworkProfileData`. Wider SSID column + reshuffled fields.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NifmWirelessSettingData {
    pub ssid_len: u8,
    pub ssid: [u8; 0x21],
    pub unk_x22: u8,
    pub pad: u8,
    pub unk_x24: u32,
    pub unk_x28: u32,
    pub passphrase: [u8; 0x41],
    pub pad2: [u8; 0x3],
}

const_assert_eq!(size_of::<NifmWirelessSettingData>(), 0x70);

/// Wire-side network-profile record returned by `GetCurrentNetworkProfile` /
/// `GetNetworkProfile` and accepted by `SetNetworkProfile`.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NifmSfNetworkProfileData {
    pub ip_setting_data: NifmIpSettingData,
    pub uuid: Uuid,
    pub network_name: [u8; 0x40],
    pub unk_x112: u8,
    pub unk_x113: u8,
    pub unk_x114: u8,
    pub unk_x115: u8,
    pub wireless_setting_data: NifmSfWirelessSettingData,
    pub pad: u8,
}

// 0xC2 + 0x10 + 0x40 + 4*u8 + 0x65 + 0x01 = 0x17C.
const_assert_eq!(size_of::<NifmSfNetworkProfileData>(), 0x17C);
const_assert_eq!(offset_of!(NifmSfNetworkProfileData, uuid), 0xC2);
const_assert_eq!(offset_of!(NifmSfNetworkProfileData, network_name), 0xD2);
const_assert_eq!(offset_of!(NifmSfNetworkProfileData, unk_x112), 0x112);

/// Application-side network-profile record. Layout matches libnx's
/// `NifmNetworkProfileData` exactly so it can be filled in by the SF→App
/// conversion routines in [`crate::cmif::general`].
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct NifmNetworkProfileData {
    pub uuid: Uuid,
    pub network_name: [u8; 0x40],
    pub unk_x50: u32,
    pub unk_x54: u32,
    pub unk_x58: u8,
    pub unk_x59: u8,
    pub pad: [u8; 2],
    pub wireless_setting_data: NifmWirelessSettingData,
    pub ip_setting_data: NifmIpSettingData,
}

// Payload ends at 0x18E; outer alignment is 4 (from `NifmWirelessSettingData`'s
// `u32` fields), so the compiler adds 2 bytes of trailing padding → 0x190.
const_assert_eq!(size_of::<NifmNetworkProfileData>(), 0x190);
const_assert_eq!(offset_of!(NifmNetworkProfileData, network_name), 0x10);
const_assert_eq!(
    offset_of!(NifmNetworkProfileData, wireless_setting_data),
    0x5C
);
const_assert_eq!(offset_of!(NifmNetworkProfileData, ip_setting_data), 0xCC);

/// Wire-side basic-info record returned by `EnumerateNetworkProfiles`.
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct NifmSfNetworkProfileBasicInfo {
    pub uuid: Uuid,
    pub network_name: [u8; 0x40],
    pub profile_type: u8,
    pub connection_type: u8,
    pub ssid_len: u8,
    pub ssid: [u8; 0x20],
    pub authentication: u8,
    pub encryption: u8,
}

// 0x10 + 0x40 + 3*u8 + 0x20 + 2*u8 = 0x75.
const_assert_eq!(size_of::<NifmSfNetworkProfileBasicInfo>(), 0x75);

/// Application-side basic-info record. libnx widens the enum fields to `u8`-typed
/// enums, pads to align, and re-uses the wire layout otherwise.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NifmNetworkProfileBasicInfo {
    pub uuid: Uuid,
    pub network_name: [u8; 0x40],
    pub profile_type: u8,
    pub connection_type: u8,
    pub ssid_len: u8,
    pub ssid: [u8; 0x20],
    pub pad: [u8; 3],
    pub authentication: u8,
    pub encryption: u8,
}

// 0x10 + 0x40 + 3*u8 + 0x20 + 0x03 + 2*u8 = 0x78.
const_assert_eq!(size_of::<NifmNetworkProfileBasicInfo>(), 0x78);

/// `ClientId` returned by `GetClientId` (cmd 1).
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NifmClientId {
    pub id: u32,
}

const_assert_eq!(size_of::<NifmClientId>(), 0x4);

/// Combined output of `GetCurrentIpConfigInfo` (cmd 15).
///
/// libnx exposes each field as a separate `u32*` out-parameter; we keep them
/// grouped here so the caller can destructure once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpConfigInfo {
    pub current_addr: NifmIpV4Address,
    pub subnet_mask: NifmIpV4Address,
    pub gateway: NifmIpV4Address,
    pub primary_dns_server: NifmIpV4Address,
    pub secondary_dns_server: NifmIpV4Address,
}

/// Output of `GetInternetConnectionStatus` (cmd 18).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InternetConnection {
    pub connection_type: crate::proto::NifmInternetConnectionType,
    /// Wi-Fi signal strength in bars (0–3).
    pub wifi_strength: u8,
    pub status: crate::proto::NifmInternetConnectionStatus,
}

/// Output of `IRequest::GetAppletInfo` (cmd 21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppletInfo {
    pub applet_id: u32,
    pub mode: u32,
    pub out_size: u32,
}

/// Mirrors libnx's `_nifmConvertSfToNetworkProfileData`.
pub fn sf_to_network_profile_data(
    input: &NifmSfNetworkProfileData,
    output: &mut NifmNetworkProfileData,
) {
    // Zero-initialise the output to match libnx's `memset(out, 0, sizeof(*out));`.
    *output = NifmNetworkProfileData::new_zeroed();

    output.uuid = input.uuid;
    output.network_name.copy_from_slice(&input.network_name);
    // libnx force-NUL-terminates the last byte.
    output.network_name[0x3F] = 0;

    output.unk_x50 = input.unk_x112 as u32;
    output.unk_x54 = input.unk_x113 as u32;
    output.unk_x58 = input.unk_x114;
    output.unk_x59 = input.unk_x115;

    let mut ssid_len = input.wireless_setting_data.ssid_len as usize;
    let max_ssid = output.wireless_setting_data.ssid.len() - 1; // 0x20
    if ssid_len > max_ssid {
        ssid_len = max_ssid;
    }
    output.wireless_setting_data.ssid_len = ssid_len as u8;
    output.wireless_setting_data.ssid[..ssid_len]
        .copy_from_slice(&input.wireless_setting_data.ssid[..ssid_len]);
    output.wireless_setting_data.unk_x22 = input.wireless_setting_data.unk_x21;
    output.wireless_setting_data.unk_x24 = input.wireless_setting_data.unk_x22 as u32;
    output.wireless_setting_data.unk_x28 = input.wireless_setting_data.unk_x23 as u32;
    output
        .wireless_setting_data
        .passphrase
        .copy_from_slice(&input.wireless_setting_data.passphrase);

    output.ip_setting_data = input.ip_setting_data;
}

/// Mirrors libnx's `_nifmConvertSfFromNetworkProfileData`.
pub fn sf_from_network_profile_data(
    input: &NifmNetworkProfileData,
    output: &mut NifmSfNetworkProfileData,
) {
    *output = NifmSfNetworkProfileData::new_zeroed();

    output.uuid = input.uuid;
    output.network_name.copy_from_slice(&input.network_name);
    output.network_name[0x3F] = 0;

    output.unk_x112 = input.unk_x50 as u8;
    output.unk_x113 = input.unk_x54 as u8;
    output.unk_x114 = input.unk_x58;
    output.unk_x115 = input.unk_x59;

    output.wireless_setting_data.ssid_len = input.wireless_setting_data.ssid_len;
    // libnx copies `sizeof(out->ssid)-1` bytes; out->ssid is [u8; 0x20].
    let copy_len = output.wireless_setting_data.ssid.len() - 1;
    output.wireless_setting_data.ssid[..copy_len]
        .copy_from_slice(&input.wireless_setting_data.ssid[..copy_len]);
    output.wireless_setting_data.unk_x21 = input.wireless_setting_data.unk_x22;
    output.wireless_setting_data.unk_x22 = input.wireless_setting_data.unk_x24 as u8;
    output.wireless_setting_data.unk_x23 = input.wireless_setting_data.unk_x28 as u8;
    output
        .wireless_setting_data
        .passphrase
        .copy_from_slice(&input.wireless_setting_data.passphrase);

    output.ip_setting_data = input.ip_setting_data;
}

/// Mirrors libnx's `_nifmConvertSfToNetworkProfileBasicInfo`.
pub fn sf_to_network_profile_basic_info(
    input: &NifmSfNetworkProfileBasicInfo,
    output: &mut NifmNetworkProfileBasicInfo,
) {
    *output = NifmNetworkProfileBasicInfo::new_zeroed();

    output.uuid = input.uuid;
    output.network_name.copy_from_slice(&input.network_name);
    output.network_name[0x3F] = 0;
    output.profile_type = input.profile_type;
    output.connection_type = input.connection_type;

    let mut ssid_len = input.ssid_len as usize;
    if ssid_len > output.ssid.len() {
        ssid_len = output.ssid.len();
    }
    output.ssid_len = ssid_len as u8;
    output.ssid[..ssid_len].copy_from_slice(&input.ssid[..ssid_len]);
    output.authentication = input.authentication;
    output.encryption = input.encryption;
}
