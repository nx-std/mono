//! The inserted game card: the handle that names it, its partitions, and what
//! the device operator reports about it.

use static_assertions::const_assert_eq;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GameCardPartition {
    Update = 0,
    Normal = 1,
    Secure = 2,
    Logo = 3,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct GameCardAttribute: u8 {
        const AUTO_BOOT = 1 << 0;
        const HISTORY_ERASE = 1 << 1;
        const REPAIR_TOOL = 1 << 2;
        const DIFFERENT_REGION_CUP_TO_TERRA_DEVICE = 1 << 3;
        const DIFFERENT_REGION_CUP_TO_GLOBAL_DEVICE = 1 << 4;
    }
}

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
pub struct GameCardHandle {
    pub value: u32,
}
const_assert_eq!(core::mem::size_of::<GameCardHandle>(), 0x4);

#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct GameCardUpdatePartitionInfo {
    pub version: u32,
    pub pad: [u8; 4],
    pub id: u64,
}
const_assert_eq!(core::mem::size_of::<GameCardUpdatePartitionInfo>(), 0x10);

#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct GameCardErrorReportInfo {
    pub game_card_crc_error_num: u16,
    pub reserved1: u16,
    pub asic_crc_error_num: u16,
    pub reserved2: u16,
    pub refresh_num: u16,
    pub reserved3: u16,
    pub retry_limit_out_num: u16,
    pub timeout_retry_num: u16,
    pub asic_reinitialize_failure_detail: u16,
    pub insertion_count: u16,
    pub removal_count: u16,
    pub asic_reinitialize_num: u16,
    pub initialize_count: u32,
    pub asic_reinitialize_failure_num: u16,
    pub awaken_failure_num: u16,
    pub reserved4: u16,
    pub refresh_succeeded_count: u16,
    pub last_read_error_page_address: u32,
    pub last_read_error_page_count: u32,
    pub awaken_count: u32,
    pub read_count_from_insert: u32,
    pub read_count_from_awaken: u32,
    pub reserved5: [u8; 8],
}
const_assert_eq!(core::mem::size_of::<GameCardErrorReportInfo>(), 0x40);

#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct OpenGameCardFileSystemIn {
    pub handle: GameCardHandle,
    pub partition: u32,
}
const_assert_eq!(core::mem::size_of::<OpenGameCardFileSystemIn>(), 0x8);

#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GetDeviceCertIn {
    pub handle: GameCardHandle,
    pub _pad: u32,
    pub buffer_size: i64,
}
const_assert_eq!(core::mem::size_of::<GetDeviceCertIn>(), 0x10);
