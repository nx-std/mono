//! Wire-format C structs shared with the `ldn:*` IPC servers.
//!
//! Layouts mirror libnx's `services/ldn.h`. Each struct has a
//! `const_assert_eq!` size check so accidental drift fails at compile time.

use core::mem::size_of;

use static_assertions::const_assert_eq;

use crate::proto::LdnScanFilterFlag;

/// IPv4 address (network-order convention is the caller's responsibility — the
/// service does no byteswap).
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
pub struct LdnIpv4Address {
    pub addr: u32,
}
const_assert_eq!(size_of::<LdnIpv4Address>(), 4);

/// IPv4 subnet mask.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct LdnSubnetMask {
    pub mask: u32,
}
const_assert_eq!(size_of::<LdnSubnetMask>(), 4);

/// 48-bit MAC address.
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
pub struct LdnMacAddress {
    pub addr: [u8; 6],
}
const_assert_eq!(size_of::<LdnMacAddress>(), 6);

/// SSID — length-prefixed string with a NUL-terminated payload.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct LdnSsid {
    /// Length of `str` excluding the NUL terminator. Must be `0x1..=0x20`.
    pub len: u8,
    /// SSID bytes including the NUL terminator. Printable ASCII (`0x20..=0x7F`).
    pub str: [u8; 0x21],
}
const_assert_eq!(size_of::<LdnSsid>(), 0x22);

/// Per-node update bookkeeping returned by `GetNetworkInfoAndHistory`.
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
pub struct LdnNodeLatestUpdate {
    pub state_change: u8,
    pub reserved: [u8; 7],
}
const_assert_eq!(size_of::<LdnNodeLatestUpdate>(), 8);

/// IP + MAC entry, used as a static accept list for `CreateNetworkPrivate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct LdnAddressEntry {
    pub ip_addr: LdnIpv4Address,
    pub mac_addr: LdnMacAddress,
    pub reserved: [u8; 2],
}
const_assert_eq!(size_of::<LdnAddressEntry>(), 0xC);

/// Per-station node information.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct LdnNodeInfo {
    pub ip_addr: LdnIpv4Address,
    pub mac_addr: LdnMacAddress,
    pub node_id: i8,
    pub is_connected: u8,
    pub user_name: [u8; 0x21],
    /// `[19.0.0+]` Platform tag (0 = NX, 1 = Ounce).
    pub platform: u8,
    pub local_communication_version: i16,
    pub reserved_x30: [u8; 0x10],
}
const_assert_eq!(size_of::<LdnNodeInfo>(), 0x40);

/// User-config payload (user-name is the only meaningful field).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct LdnUserConfig {
    pub user_name: [u8; 0x21],
    pub reserved: [u8; 0xF],
}
const_assert_eq!(size_of::<LdnUserConfig>(), 0x30);

/// Intent-id pair — local communication id + scene id.
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
pub struct LdnIntentId {
    pub local_communication_id: i64,
    pub reserved_x8: [u8; 2],
    pub scene_id: u16,
    pub reserved_xc: [u8; 4],
}
const_assert_eq!(size_of::<LdnIntentId>(), 0x10);

/// Per-network random nonce used as the SSID basis.
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
pub struct LdnSessionId {
    pub random: [u8; 0x10],
}
const_assert_eq!(size_of::<LdnSessionId>(), 0x10);

/// Full network identity (intent + session).
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
pub struct LdnNetworkId {
    pub intent_id: LdnIntentId,
    pub session_id: LdnSessionId,
}
const_assert_eq!(size_of::<LdnNetworkId>(), 0x20);

/// On-wire common network info (BSSID + SSID + radio params).
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct LdnCommonNetworkInfo {
    pub bssid: LdnMacAddress,
    pub ssid: LdnSsid,
    pub channel: i16,
    pub link_level: i8,
    pub network_type: u8,
    pub reserved: [u8; 4],
}
const_assert_eq!(size_of::<LdnCommonNetworkInfo>(), 0x30);

/// Network-info blob returned by `GetNetworkInfo` / `Scan`.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct LdnNetworkInfo {
    pub network_id: LdnNetworkId,
    pub common: LdnCommonNetworkInfo,
    pub server_random: [u8; 0x10],
    pub security_mode: u16,
    pub station_accept_policy: u8,
    pub version: u8,
    pub reserved_x14: [u8; 2],
    pub node_count_max: i8,
    pub node_count: u8,
    pub nodes: [LdnNodeInfo; 8],
    pub reserved_x218: [u8; 2],
    pub advertise_data_size: u16,
    pub advertise_data: [u8; 0x180],
    pub reserved_x39c: [u8; 0x8C],
    pub reserved_x428: u64,
}
const_assert_eq!(size_of::<LdnNetworkInfo>(), 0x480);

/// Scan filter sent to `Scan` / `ScanPrivate`.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct LdnScanFilter {
    pub network_id: LdnNetworkId,
    pub network_type: u32,
    pub bssid: LdnMacAddress,
    pub ssid: LdnSsid,
    pub reserved: [u8; 0x10],
    /// Bitmask of [`LdnScanFilterFlag`] values.
    pub flags: LdnScanFilterFlag,
}
const_assert_eq!(size_of::<LdnScanFilter>(), 0x60);

/// Security material used to derive the SSID / passphrase.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct LdnSecurityConfig {
    pub security_mode: u16,
    pub passphrase_size: u16,
    pub passphrase: [u8; 0x40],
}
const_assert_eq!(size_of::<LdnSecurityConfig>(), 0x44);

/// Random server nonce + session-id pair.
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
pub struct LdnSecurityParameter {
    pub server_random: [u8; 0x10],
    pub session_id: LdnSessionId,
}
const_assert_eq!(size_of::<LdnSecurityParameter>(), 0x20);

/// Network-config payload for `CreateNetwork` / `CreateNetworkPrivate` /
/// `ConnectPrivate`.
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
pub struct LdnNetworkConfig {
    pub intent_id: LdnIntentId,
    pub channel: i16,
    pub node_count_max: i8,
    pub reserved_x13: u8,
    pub local_communication_version: i16,
    pub reserved_x16: [u8; 0xA],
}
const_assert_eq!(size_of::<LdnNetworkConfig>(), 0x20);

/// Settings for `EnableActionFrame`.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct LdnActionFrameSettings {
    pub local_communication_id: i64,
    pub reserved: [u8; 0x34],
    pub security_mode: u16,
    pub passphrase_size: u16,
    pub passphrase: [u8; 0x40],
}
const_assert_eq!(size_of::<LdnActionFrameSettings>(), 0x80);
