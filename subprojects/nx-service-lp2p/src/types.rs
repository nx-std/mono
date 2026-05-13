//! Wire-layout types for the LP2P service.

use static_assertions::const_assert_eq;

/// MAC address (6 bytes).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Lp2pMacAddress {
    pub addr: [u8; 6],
}
const_assert_eq!(size_of::<Lp2pMacAddress>(), 0x6);

/// Group identifier (BSSID, 6 bytes).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Lp2pGroupId {
    pub id: [u8; 6],
}
const_assert_eq!(size_of::<Lp2pGroupId>(), 0x6);

/// Group information used for creating/joining groups and scanning.
///
/// When used as input for [`Scan`](crate::Lp2pNetworkService::scan), only
/// `supported_platform`, `priority`, `frequency`, `channel`,
/// `preshared_key_binary_size`, and `preshared_key` are read by the service.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Lp2pGroupInfo {
    pub unk_x0: [u8; 0x10],
    pub local_communication_id: u64,
    pub group_id: Lp2pGroupId,
    pub service_name: [u8; 0x21],
    pub flags_count: i8,
    pub flags: [i8; 0x40],
    pub supported_platform: u8,
    pub member_count_max: i8,
    pub unk_x82: u8,
    pub unk_x83: u8,
    pub frequency: u16,
    pub channel: i16,
    pub network_mode: u8,
    pub performance_requirement: u8,
    pub security_type: u8,
    pub static_aes_key_index: i8,
    pub unk_x8c: u8,
    pub priority: u8,
    pub stealth_enabled: u8,
    pub unk_x8f: u8,
    pub unk_x90: [u8; 0x130],
    pub preshared_key_binary_size: u8,
    pub preshared_key: [u8; 0x3F],
}
const_assert_eq!(size_of::<Lp2pGroupInfo>(), 0x200);

impl Default for Lp2pGroupInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl Lp2pGroupInfo {
    /// Creates a default `Lp2pGroupInfo` for use with
    /// [`create_group`](crate::Lp2pNetworkService::create_group) /
    /// [`join`](crate::Lp2pNetworkServiceMonitor::join).
    pub fn new() -> Self {
        let mut info = Self::zeroed();
        info.flags_count = 1;
        info.flags[0] = 1;
        info.supported_platform = 1;
        info.unk_x82 = 0x2;
        info.network_mode = 1;
        info.performance_requirement = 3;
        info.priority = 90;
        info
    }

    /// Creates a default `Lp2pGroupInfo` for use with
    /// [`scan`](crate::Lp2pNetworkService::scan).
    pub fn new_scan() -> Self {
        let mut info = Self::zeroed();
        info.supported_platform = 1;
        info.priority = 90;
        info
    }

    fn zeroed() -> Self {
        // SAFETY: Lp2pGroupInfo is #[repr(C)] with no padding invariants;
        // all-zeros is a valid bit pattern.
        unsafe { core::mem::zeroed() }
    }
}

/// Scan result entry.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Lp2pScanResult {
    pub group_info: Lp2pGroupInfo,
    pub unk_x200: u8,
    pub unk_x201: [u8; 0x5],
    pub advertise_data_size: u16,
    pub advertise_data: [u8; 0x80],
    pub unk_x288: [u8; 0x78],
}
const_assert_eq!(size_of::<Lp2pScanResult>(), 0x300);

/// Node information (member/owner).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Lp2pNodeInfo {
    pub ip_addr: [u8; 0x20],
    pub unk_x20: [u8; 0x4],
    pub mac_addr: Lp2pMacAddress,
    pub unk_x2a: [u8; 0x56],
}
const_assert_eq!(size_of::<Lp2pNodeInfo>(), 0x80);

/// IP configuration (IPv4 only).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Lp2pIpConfig {
    pub unk_x0: [u8; 0x20],
    pub ip_addr: [u8; 0x20],
    pub subnet_mask: [u8; 0x20],
    pub gateway: [u8; 0x20],
    pub unk_x80: [u8; 0x80],
}
const_assert_eq!(size_of::<Lp2pIpConfig>(), 0x100);

// ---------------------------------------------------------------------------
// IPC payload structs (crate-internal)
// ---------------------------------------------------------------------------

/// Input payload for CreateNetworkService (cmd 0).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct CreateNetworkServiceIn {
    pub inval: u32,
    pub _pad: u32,
    pub pid_placeholder: u64,
}
const_assert_eq!(size_of::<CreateNetworkServiceIn>(), 0x10);

/// Input payload for SendToOtherGroup (cmd 1536).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct SendToOtherGroupIn {
    pub addr: Lp2pMacAddress,
    pub group_id: Lp2pGroupId,
    pub frequency: i16,
    pub channel: i16,
    pub flags: u32,
}
const_assert_eq!(size_of::<SendToOtherGroupIn>(), 0x14);

/// Output payload for RecvFromOtherGroup (cmd 1544).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct RecvFromOtherGroupOut {
    pub addr: Lp2pMacAddress,
    pub unk0: u16,
    pub unk1: i16,
    pub _pad: u16,
    pub out_size: u32,
    pub unk2: i32,
}
const_assert_eq!(size_of::<RecvFromOtherGroupOut>(), 0x14);

/// Output payload for GetAdvertiseData / GetAdvertiseData2 (cmds 280/281).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetAdvertiseDataOut {
    pub transfer_size: u16,
    pub original_size: u16,
}
const_assert_eq!(size_of::<GetAdvertiseDataOut>(), 0x4);

/// Result data from [`recv_from_other_group`](crate::Lp2pNetworkService::recv_from_other_group).
pub struct RecvFromOtherGroupResult {
    pub addr: Lp2pMacAddress,
    pub unk0: u16,
    pub unk1: i32,
    pub out_size: u64,
    pub unk2: i32,
}

/// Result data from [`get_advertise_data`](crate::Lp2pNetworkServiceMonitor::get_advertise_data).
pub struct AdvertiseDataResult {
    pub transfer_size: u16,
    pub original_size: u16,
}
