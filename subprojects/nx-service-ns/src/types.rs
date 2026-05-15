//! Wire-layout types for the NS service family.

use core::mem::size_of;

use static_assertions::const_assert_eq;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ShellEvent {
    None = 0,
    Exit = 1,
    Start = 2,
    Crash = 3,
    Debug = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ApplicationControlSource {
    CacheOnly = 0,
    Storage = 1,
    StorageOnly = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BackgroundNetworkUpdateState {
    None = 0,
    Downloading = 1,
    Ready = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LatestSystemUpdate {
    UpToDate = 0,
    Downloaded = 1,
    NeedsDownload = 2,
}

// ---------------------------------------------------------------------------
// Public wire-layout types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ApplicationControlData {
    pub nacp: [u8; 0x4000],
    pub icon: [u8; 0x20000],
}
const_assert_eq!(size_of::<ApplicationControlData>(), 0x24000);

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ApplicationOccupiedSize {
    pub data: [u8; 0x80],
}
const_assert_eq!(size_of::<ApplicationOccupiedSize>(), 0x80);

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ApplicationContentMetaStatus {
    pub meta_type: u8,
    pub storage_id: u8,
    pub rights_check: u8,
    pub reserved: u8,
    pub version: u32,
    pub application_id: u64,
}
const_assert_eq!(size_of::<ApplicationContentMetaStatus>(), 0x10);

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ApplicationRecord {
    pub application_id: u64,
    pub last_event: u8,
    pub attributes: u8,
    pub reserved: [u8; 6],
    pub last_updated: u64,
}
const_assert_eq!(size_of::<ApplicationRecord>(), 0x18);

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ProgressForDeleteUserSaveDataAll {
    pub data: [u8; 0x28],
}
const_assert_eq!(size_of::<ProgressForDeleteUserSaveDataAll>(), 0x28);

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ApplicationViewDeprecated {
    pub application_id: u64,
    pub unk_x8: [u8; 0x4],
    pub flags: u32,
    pub unk_x10: [u8; 0x10],
    pub unk_x20: u32,
    pub unk_x24: u16,
    pub unk_x26: [u8; 0x2],
    pub unk_x28: [u8; 0x10],
    pub unk_x38: u32,
    pub unk_x3c: u8,
    pub unk_x3d: [u8; 3],
}
const_assert_eq!(size_of::<ApplicationViewDeprecated>(), 0x40);

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ApplicationView {
    pub application_id: u64,
    pub unk_x8: [u8; 0x4],
    pub flags: u32,
    pub unk_x10: [u8; 0x10],
    pub unk_x20: u32,
    pub unk_x24: u16,
    pub unk_x26: [u8; 0x2],
    pub unk_x28: [u8; 0x8],
    pub unk_x30: [u8; 0x10],
    pub unk_x40: u32,
    pub unk_x44: u8,
    pub unk_x45: [u8; 0xb],
}
const_assert_eq!(size_of::<ApplicationView>(), 0x50);

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PromotionInfo {
    pub start_timestamp: u64,
    pub end_timestamp: u64,
    pub remaining_time: i64,
    pub unk_x18: [u8; 0x4],
    pub flags: u8,
    pub pad: [u8; 3],
}
const_assert_eq!(size_of::<PromotionInfo>(), 0x20);

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ApplicationViewWithPromotionInfo {
    pub view: ApplicationView,
    pub promotion: PromotionInfo,
}
const_assert_eq!(size_of::<ApplicationViewWithPromotionInfo>(), 0x70);

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct LaunchProperties {
    pub program_id: u64,
    pub version: u32,
    pub storage_id: u8,
    pub index: u8,
    pub is_application: u8,
}
const_assert_eq!(size_of::<LaunchProperties>(), 0x0F);

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ShellEventInfo {
    pub event: u32,
    pub pad: u32,
    pub process_id: u64,
}
const_assert_eq!(size_of::<ShellEventInfo>(), 0x10);

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SystemUpdateProgress {
    pub current_size: i64,
    pub total_size: i64,
}
const_assert_eq!(size_of::<SystemUpdateProgress>(), 0x10);

pub type ReceiveApplicationProgress = SystemUpdateProgress;
pub type SendApplicationProgress = SystemUpdateProgress;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct EulaDataPath {
    pub path: [u8; 0x100],
}
const_assert_eq!(size_of::<EulaDataPath>(), 0x100);

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SystemDeliveryInfoData {
    pub system_delivery_protocol_version: u32,
    pub application_delivery_protocol_version: u32,
    pub has_exfat: u8,
    pub reserved: [u8; 0x3],
    pub system_update_version: u32,
    pub old_system_update_id: u64,
    pub firmware_variation_id: u8,
    pub updatable_firmware_group_id: u8,
    pub platform_region: u8,
    pub system_delivery_info_platform: u8,
    pub system_update_id_flag: u8,
    pub pad: [u8; 0x3],
    pub system_update_id: u64,
    pub reserved_x28: [u8; 0xb8],
}
const_assert_eq!(size_of::<SystemDeliveryInfoData>(), 0xE0);

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SystemDeliveryInfo {
    pub data: SystemDeliveryInfoData,
    pub hmac: [u8; 0x20],
}
const_assert_eq!(size_of::<SystemDeliveryInfo>(), 0x100);

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ApplicationDeliveryInfoData {
    pub application_delivery_protocol_version: u32,
    pub pad: [u8; 0x4],
    pub application_id: u64,
    pub application_version: u32,
    pub required_application_version: u32,
    pub required_system_version: u32,
    pub attributes: u32,
    pub platform: u8,
    pub proper_program_exists: u8,
    pub reserved: [u8; 0xbe],
}
const_assert_eq!(size_of::<ApplicationDeliveryInfoData>(), 0xE0);

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ApplicationDeliveryInfo {
    pub data: ApplicationDeliveryInfoData,
    pub hmac: [u8; 0x20],
}
const_assert_eq!(size_of::<ApplicationDeliveryInfo>(), 0x100);

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ApplicationRightsOnClient {
    pub application_id: u64,
    pub uid: [u8; 0x10],
    pub flags_x18: u8,
    pub flags_x19: u8,
    pub unk_x1a: [u8; 0x6],
}
const_assert_eq!(size_of::<ApplicationRightsOnClient>(), 0x20);

#[derive(Clone, Copy)]
#[repr(C)]
pub struct DownloadTaskStatus {
    pub data: [u8; 0x20],
}
const_assert_eq!(size_of::<DownloadTaskStatus>(), 0x20);

/// NcmContentMetaKey — matches libnx's NcmContentMetaKey layout (0x10 bytes).
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct NcmContentMetaKey {
    pub id: u64,
    pub version: u32,
    pub meta_type: u8,
    pub install_type: u8,
    pub padding: [u8; 2],
}
const_assert_eq!(size_of::<NcmContentMetaKey>(), 0x10);

/// AccountUid — 16-byte account identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct AccountUid {
    pub uid: [u64; 2],
}
const_assert_eq!(size_of::<AccountUid>(), 0x10);

// ---------------------------------------------------------------------------
// Internal input/output structs (pub(crate) only)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct IsEntityMovableIn {
    pub(crate) storage_id: u8,
    pub(crate) pad: [u8; 7],
    pub(crate) application_id: u64,
}
const_assert_eq!(size_of::<IsEntityMovableIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct StorageIdS64Out {
    pub(crate) storage_id: u8,
    pub(crate) pad: [u8; 7],
    pub(crate) size: i64,
}
const_assert_eq!(size_of::<StorageIdS64Out>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct SetTerminateResultIn {
    pub(crate) result: u32,
    pub(crate) pad: u32,
    pub(crate) application_id: u64,
}
const_assert_eq!(size_of::<SetTerminateResultIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct DeleteUserSystemSaveDataIn {
    pub(crate) uid: AccountUid,
    pub(crate) system_save_data_id: u64,
}
const_assert_eq!(size_of::<DeleteUserSystemSaveDataIn>(), 0x18);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct DeleteSaveDataIn {
    pub(crate) save_data_space_id: u8,
    pub(crate) pad: [u8; 7],
    pub(crate) save_data_id: u64,
}
const_assert_eq!(size_of::<DeleteSaveDataIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GameCardRegistrationGoldPointIn {
    pub(crate) uid: AccountUid,
    pub(crate) application_id: u64,
}
const_assert_eq!(size_of::<GameCardRegistrationGoldPointIn>(), 0x18);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct RegisterGameCardIn {
    pub(crate) inval: i32,
    pub(crate) pad: u32,
    pub(crate) uid: AccountUid,
    pub(crate) application_id: u64,
}
const_assert_eq!(size_of::<RegisterGameCardIn>(), 0x20);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct IsUpdateRequestedOut {
    pub(crate) flag: u8,
    pub(crate) pad: [u8; 3],
    pub(crate) out: u32,
}
const_assert_eq!(size_of::<IsUpdateRequestedOut>(), 0x08);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct CleanupUnavailableAddOnContentsIn {
    pub(crate) application_id: u64,
    pub(crate) uid: AccountUid,
}
const_assert_eq!(size_of::<CleanupUnavailableAddOnContentsIn>(), 0x18);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct EstimateSizeToMoveIn {
    pub(crate) storage_id: u8,
    pub(crate) pad: [u8; 3],
    pub(crate) flags: u32,
    pub(crate) application_id: u64,
}
const_assert_eq!(size_of::<EstimateSizeToMoveIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ContentMetaStatusIn {
    pub(crate) index: i32,
    pub(crate) pad: u32,
    pub(crate) application_id: u64,
}
const_assert_eq!(size_of::<ContentMetaStatusIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetApplicationDeliveryInfoIn {
    pub(crate) attr: u32,
    pub(crate) pad: u32,
    pub(crate) application_id: u64,
}
const_assert_eq!(size_of::<GetApplicationDeliveryInfoIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct RequestReceiveApplicationIn {
    pub(crate) storage_id: u8,
    pub(crate) pad0: u8,
    pub(crate) port: u16,
    pub(crate) addr: u32,
    pub(crate) application_id: u64,
}
const_assert_eq!(size_of::<RequestReceiveApplicationIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct RequestSendApplicationIn {
    pub(crate) port: u16,
    pub(crate) pad: u16,
    pub(crate) addr: u32,
    pub(crate) application_id: u64,
}
const_assert_eq!(size_of::<RequestSendApplicationIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ListNotCommittedContentMetaIn {
    pub(crate) unk: i32,
    pub(crate) pad: u32,
    pub(crate) application_id: u64,
}
const_assert_eq!(size_of::<ListNotCommittedContentMetaIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetApplicationRightsOnClientIn {
    pub(crate) flags: u32,
    pub(crate) pad: u32,
    pub(crate) application_id: u64,
    pub(crate) uid: AccountUid,
}
const_assert_eq!(size_of::<GetApplicationRightsOnClientIn>(), 0x20);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ControlDataSourceAppIdIn {
    pub(crate) source: u8,
    pub(crate) pad: [u8; 7],
    pub(crate) application_id: u64,
}
const_assert_eq!(size_of::<ControlDataSourceAppIdIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ControlData2In {
    pub(crate) source: u8,
    pub(crate) flag1: u8,
    pub(crate) acd_idx: u8,
    pub(crate) pad: [u8; 5],
    pub(crate) application_id: u64,
}
const_assert_eq!(size_of::<ControlData2In>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct StorageSizesOut {
    pub(crate) total_space_size: i64,
    pub(crate) free_space_size: i64,
}
const_assert_eq!(size_of::<StorageSizesOut>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct RequestSendReceiveSystemUpdateIn {
    pub(crate) port: u16,
    pub(crate) pad: u16,
    pub(crate) addr: u32,
}
const_assert_eq!(size_of::<RequestSendReceiveSystemUpdateIn>(), 0x08);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ListApplicationTitleIn {
    pub(crate) source: u8,
    pub(crate) pad: [u8; 7],
    pub(crate) tmem_size: u64,
}
const_assert_eq!(size_of::<ListApplicationTitleIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct VerifyApplicationDeprecatedIn {
    pub(crate) application_id: u64,
    pub(crate) tmem_size: u64,
}
const_assert_eq!(size_of::<VerifyApplicationDeprecatedIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct VerifyApplicationIn {
    pub(crate) unk: u32,
    pub(crate) pad: u32,
    pub(crate) application_id: u64,
    pub(crate) tmem_size: u64,
}
const_assert_eq!(size_of::<VerifyApplicationIn>(), 0x18);

// ns:dev input structs

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct NsdevLaunchProgramIn {
    pub(crate) flags: u32,
    pub(crate) pad: u32,
    pub(crate) properties: LaunchProperties,
    pub(crate) pad2: u8,
}
const_assert_eq!(size_of::<NsdevLaunchProgramIn>(), 0x18);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct NsdevLaunchApplicationForDevelopIn {
    pub(crate) flags: u32,
    pub(crate) pad: u32,
    pub(crate) application_id: u64,
}
const_assert_eq!(size_of::<NsdevLaunchApplicationForDevelopIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct NsdevLaunchApplicationWithStorageIdIn {
    pub(crate) app_storage_id: u8,
    pub(crate) patch_storage_id: u8,
    pub(crate) pad: [u8; 2],
    pub(crate) flags: u32,
    pub(crate) application_id: u64,
}
const_assert_eq!(size_of::<NsdevLaunchApplicationWithStorageIdIn>(), 0x10);
