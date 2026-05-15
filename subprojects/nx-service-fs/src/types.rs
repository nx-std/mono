use static_assertions::const_assert_eq;

pub const FS_MAX_PATH: usize = 0x301;

pub const FS_SAVEDATA_CURRENT_APPLICATIONID: u64 = 0;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum DirEntryType {
    Dir = 0,
    File = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ContentStorageId {
    System = 0,
    User = 1,
    SdCard = 2,
    System0 = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CustomStorageId {
    System = 0,
    SdCard = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ImageDirectoryId {
    Nand = 0,
    Sd = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum SaveDataSpaceId {
    System = 0,
    User = 1,
    SdSystem = 2,
    Temporary = 3,
    SdUser = 4,
    ProperSystem = 100,
    SafeMode = 101,
    All = -1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SaveDataType {
    System = 0,
    Account = 1,
    Bcat = 2,
    Device = 3,
    Temporary = 4,
    Cache = 5,
    SystemBcat = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SaveDataRank {
    Primary = 0,
    Secondary = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SaveDataMetaType {
    None = 0,
    Thumbnail = 1,
    ExtensionContext = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GameCardPartition {
    Update = 0,
    Normal = 1,
    Secure = 2,
    Logo = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OperationId {
    Clear = 0,
    ClearSignature = 1,
    InvalidateCache = 2,
    QueryRange = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BisPartitionId {
    BootPartition1Root = 0,
    BootPartition2Root = 10,
    UserDataRoot = 20,
    BootConfigAndPackage2Part1 = 21,
    BootConfigAndPackage2Part2 = 22,
    BootConfigAndPackage2Part3 = 23,
    BootConfigAndPackage2Part4 = 24,
    BootConfigAndPackage2Part5 = 25,
    BootConfigAndPackage2Part6 = 26,
    CalibrationBinary = 27,
    CalibrationFile = 28,
    SafeMode = 29,
    User = 30,
    System = 31,
    SystemProperEncryption = 32,
    SystemProperPartition = 33,
    SignedSystemPartitionOnSafeMode = 34,
    DeviceTreeBlob = 35,
    System0 = 36,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FileSystemType {
    Logo = 2,
    ContentControl = 3,
    ContentManual = 4,
    ContentMeta = 5,
    ContentData = 6,
    ApplicationPackage = 7,
    RegisteredUpdate = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FileSystemQueryId {
    SetConcatenationFileAttribute = 0,
    IsValidSignedSystemPartitionOnSdCard = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Priority {
    Normal = 0,
    Realtime = 1,
    Low = 2,
    Background = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ContentAttributes {
    None = 0x0,
    All = 0xF,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NcmStorageId {
    None = 0,
    Host = 1,
    GameCard = 2,
    BuiltinSystem = 3,
    BuiltinUser = 4,
    SdCard = 5,
    Any = 6,
}

// ---------------------------------------------------------------------------
// Bitflags
// ---------------------------------------------------------------------------

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenMode: u32 {
        const READ   = 1 << 0;
        const WRITE  = 1 << 1;
        const APPEND = 1 << 2;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CreateOption: u32 {
        const BIG_FILE = 1 << 0;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DirOpenMode: u32 {
        const READ_DIRS    = 1 << 0;
        const READ_FILES   = 1 << 1;
        const NO_FILE_SIZE = 1 << 31;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ReadOption: u32 {
        const NONE = 0;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct WriteOption: u32 {
        const NONE  = 0;
        const FLUSH = 1 << 0;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SaveDataFlags: u32 {
        const KEEP_AFTER_RESETTING_SYSTEM_SAVE_DATA = 1 << 0;
        const KEEP_AFTER_REFURBISHMENT = 1 << 1;
        const KEEP_AFTER_RESETTING_SYSTEM_SAVE_DATA_WITHOUT_USER_SAVE_DATA = 1 << 2;
        const NEEDS_SECURE_DELETE = 1 << 3;
    }
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

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MountHostOption: u32 {
        const NONE = 0;
        const PSEUDO_CASE_SENSITIVE = 1 << 0;
    }
}

// ---------------------------------------------------------------------------
// Wire-layout structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RightsId {
    pub c: [u8; 0x10],
}
const_assert_eq!(core::mem::size_of::<RightsId>(), 0x10);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AccountUid {
    pub uid: [u64; 2],
}
const_assert_eq!(core::mem::size_of::<AccountUid>(), 0x10);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DirectoryEntry {
    pub name: [u8; FS_MAX_PATH],
    pub pad: [u8; 3],
    pub entry_type: i8,
    pub pad2: [u8; 3],
    pub file_size: i64,
}
const_assert_eq!(core::mem::size_of::<DirectoryEntry>(), 0x310);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SaveDataAttribute {
    pub application_id: u64,
    pub uid: AccountUid,
    pub system_save_data_id: u64,
    pub save_data_type: u8,
    pub save_data_rank: u8,
    pub save_data_index: u16,
    pub pad_x24: u32,
    pub unk_x28: u64,
    pub unk_x30: u64,
    pub unk_x38: u64,
}
const_assert_eq!(core::mem::size_of::<SaveDataAttribute>(), 0x40);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SaveDataExtraData {
    pub attr: SaveDataAttribute,
    pub owner_id: u64,
    pub timestamp: u64,
    pub flags: u32,
    pub unk_x54: u32,
    pub data_size: i64,
    pub journal_size: i64,
    pub commit_id: u64,
    pub unused: [u8; 0x190],
}
const_assert_eq!(core::mem::size_of::<SaveDataExtraData>(), 0x200);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SaveDataMetaInfo {
    pub size: u32,
    pub meta_type: u8,
    pub reserved: [u8; 0x0B],
}
const_assert_eq!(core::mem::size_of::<SaveDataMetaInfo>(), 0x10);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SaveDataCreationInfo {
    pub save_data_size: i64,
    pub journal_size: i64,
    pub available_size: u64,
    pub owner_id: u64,
    pub flags: u32,
    pub save_data_space_id: u8,
    pub unk: u8,
    pub padding: [u8; 0x1a],
}
const_assert_eq!(core::mem::size_of::<SaveDataCreationInfo>(), 0x40);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SaveDataInfo {
    pub save_data_id: u64,
    pub save_data_space_id: u8,
    pub save_data_type: u8,
    pub pad: [u8; 6],
    pub uid: AccountUid,
    pub system_save_data_id: u64,
    pub application_id: u64,
    pub size: u64,
    pub save_data_index: u16,
    pub save_data_rank: u8,
    pub unk_x3b: [u8; 0x25],
}
const_assert_eq!(core::mem::size_of::<SaveDataInfo>(), 0x60);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SaveDataFilter {
    pub filter_by_application_id: u8,
    pub filter_by_save_data_type: u8,
    pub filter_by_user_id: u8,
    pub filter_by_system_save_data_id: u8,
    pub filter_by_index: u8,
    pub save_data_rank: u8,
    pub padding: [u8; 2],
    pub attr: SaveDataAttribute,
}
const_assert_eq!(core::mem::size_of::<SaveDataFilter>(), 0x48);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TimeStampRaw {
    pub created: u64,
    pub modified: u64,
    pub accessed: u64,
    pub is_valid: u8,
    pub padding: [u8; 7],
}
const_assert_eq!(core::mem::size_of::<TimeStampRaw>(), 0x20);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ArchiveMacKey {
    pub key: [u8; 0x10],
}
const_assert_eq!(core::mem::size_of::<ArchiveMacKey>(), 0x10);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GameCardHandle {
    pub value: u32,
}
const_assert_eq!(core::mem::size_of::<GameCardHandle>(), 0x4);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GameCardUpdatePartitionInfo {
    pub version: u32,
    pub pad: [u8; 4],
    pub id: u64,
}
const_assert_eq!(core::mem::size_of::<GameCardUpdatePartitionInfo>(), 0x10);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RangeInfo {
    pub aes_ctr_key_type: u32,
    pub speed_emulation_type: u32,
    pub reserved: [u32; 0x38 / 4],
}
const_assert_eq!(core::mem::size_of::<RangeInfo>(), 0x40);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FileSystemAttribute {
    pub directory_name_length_max_has_value: u8,
    pub file_name_length_max_has_value: u8,
    pub directory_path_length_max_has_value: u8,
    pub file_path_length_max_has_value: u8,
    pub utf16_create_directory_path_length_max_has_value: u8,
    pub utf16_delete_directory_path_length_max_has_value: u8,
    pub utf16_rename_source_directory_path_length_max_has_value: u8,
    pub utf16_rename_destination_directory_path_length_max_has_value: u8,
    pub utf16_open_directory_path_length_max_has_value: u8,
    pub utf16_directory_name_length_max_has_value: u8,
    pub utf16_file_name_length_max_has_value: u8,
    pub utf16_directory_path_length_max_has_value: u8,
    pub utf16_file_path_length_max_has_value: u8,
    pub reserved1: [u8; 0x1B],
    pub directory_name_length_max: i32,
    pub file_name_length_max: i32,
    pub directory_path_length_max: i32,
    pub file_path_length_max: i32,
    pub utf16_create_directory_path_length_max: i32,
    pub utf16_delete_directory_path_length_max: i32,
    pub utf16_rename_source_directory_path_length_max: i32,
    pub utf16_rename_destination_directory_path_length_max: i32,
    pub utf16_open_directory_path_length_max: i32,
    pub utf16_directory_name_length_max: i32,
    pub utf16_file_name_length_max: i32,
    pub utf16_directory_path_length_max: i32,
    pub utf16_file_path_length_max: i32,
    pub reserved2: [u8; 0x64],
}
const_assert_eq!(core::mem::size_of::<FileSystemAttribute>(), 0xC0);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StorageErrorInfo {
    pub num_activation_failures: u32,
    pub num_activation_error_corrections: u32,
    pub num_read_write_failures: u32,
    pub num_read_write_error_corrections: u32,
}
const_assert_eq!(core::mem::size_of::<StorageErrorInfo>(), 0x10);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FatFatError {
    pub error: i32,
    pub extra_error: i32,
    pub drive_id: i32,
    pub name: [u8; 16],
    pub reserved: [u8; 4],
}
const_assert_eq!(core::mem::size_of::<FatFatError>(), 0x20);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FatFatReportInfo1 {
    pub open_file_peak_count: u16,
    pub open_directory_peak_count: u16,
}
const_assert_eq!(core::mem::size_of::<FatFatReportInfo1>(), 0x4);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FatFatReportInfo2 {
    pub open_unique_file_entry_peak_count: u16,
    pub open_unique_directory_entry_peak_count: u16,
}
const_assert_eq!(core::mem::size_of::<FatFatReportInfo2>(), 0x4);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FatFatSafeInfo {
    pub result: u32,
    pub error_number: u32,
    pub safe_error_number: u32,
}
const_assert_eq!(core::mem::size_of::<FatFatSafeInfo>(), 0xC);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FileSystemProxyErrorInfo {
    pub rom_fs_remount_for_data_corruption_count: u32,
    pub rom_fs_unrecoverable_data_corruption_by_remount_count: u32,
    pub fat_fs_error: FatFatError,
    pub rom_fs_recovered_by_invalidate_cache_count: u32,
    pub save_data_index_count: u32,
    pub bis_system_fat_report_info_1: FatFatReportInfo1,
    pub bis_user_fat_report_info_1: FatFatReportInfo1,
    pub sd_card_fat_report_info_1: FatFatReportInfo1,
    pub bis_system_fat_report_info_2: FatFatReportInfo2,
    pub bis_user_fat_report_info_2: FatFatReportInfo2,
    pub sd_card_fat_report_info_2: FatFatReportInfo2,
    pub rom_fs_deep_retry_start_count: u32,
    pub rom_fs_unrecoverable_by_game_card_access_failed_count: u32,
    pub bis_system_fat_safe_info: FatFatSafeInfo,
    pub bis_user_fat_safe_info: FatFatSafeInfo,
    pub reserved: [u8; 0x18],
}
const_assert_eq!(core::mem::size_of::<FileSystemProxyErrorInfo>(), 0x80);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryReportInfo {
    pub pooled_buffer_peak_free_size: u64,
    pub pooled_buffer_retried_count: u64,
    pub pooled_buffer_reduce_allocation_count: u64,
    pub buffer_manager_peak_free_size: u64,
    pub buffer_manager_retried_count: u64,
    pub exp_heap_peak_free_size: u64,
    pub buffer_pool_peak_free_size: u64,
    pub patrol_read_allocate_buffer_success_count: u64,
    pub patrol_read_allocate_buffer_failure_count: u64,
    pub buffer_manager_peak_total_allocatable_size: u64,
    pub buffer_pool_max_allocate_size: u64,
    pub pooled_buffer_failed_ideal_allocation_count_on_async_access: u64,
    pub reserved: [u8; 0x20],
}
const_assert_eq!(core::mem::size_of::<MemoryReportInfo>(), 0x80);

#[derive(Debug, Clone, Copy)]
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

// ---------------------------------------------------------------------------
// IPC input structs (anonymous in libnx, named here for dispatch)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenFileSystemWithPatchIn {
    pub fs_type: u32,
    pub id: u64,
}
const_assert_eq!(core::mem::size_of::<OpenFileSystemWithPatchIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenFileSystemWithIdIn {
    pub fs_type: u32,
    pub id: u64,
}
const_assert_eq!(core::mem::size_of::<OpenFileSystemWithIdIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenFileSystemWithIdV16In {
    pub attr: u8,
    pub _pad: [u8; 3],
    pub fs_type: u32,
    pub id: u64,
}
const_assert_eq!(core::mem::size_of::<OpenFileSystemWithIdV16In>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct DeleteSaveDataBySpaceIdIn {
    pub save_data_space_id: u8,
    pub _pad: [u8; 7],
    pub save_id: u64,
}
const_assert_eq!(core::mem::size_of::<DeleteSaveDataBySpaceIdIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct DeleteSaveDataByAttributeIn {
    pub save_data_space_id: u8,
    pub _pad: [u8; 7],
    pub attr: SaveDataAttribute,
}
const_assert_eq!(core::mem::size_of::<DeleteSaveDataByAttributeIn>(), 0x48);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct CreateSaveDataIn {
    pub attr: SaveDataAttribute,
    pub creation_info: SaveDataCreationInfo,
    pub meta: SaveDataMetaInfo,
}
const_assert_eq!(core::mem::size_of::<CreateSaveDataIn>(), 0x90);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct CreateSaveDataBySystemIdIn {
    pub attr: SaveDataAttribute,
    pub creation_info: SaveDataCreationInfo,
}
const_assert_eq!(core::mem::size_of::<CreateSaveDataBySystemIdIn>(), 0x80);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenGameCardFileSystemIn {
    pub handle: GameCardHandle,
    pub partition: u32,
}
const_assert_eq!(core::mem::size_of::<OpenGameCardFileSystemIn>(), 0x8);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ExtendSaveDataIn {
    pub save_data_space_id: u8,
    pub pad: [u8; 7],
    pub save_id: u64,
    pub data_size: i64,
    pub journal_size: i64,
}
const_assert_eq!(core::mem::size_of::<ExtendSaveDataIn>(), 0x20);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenSaveDataIn {
    pub save_data_space_id: u8,
    pub pad: [u8; 7],
    pub attr: SaveDataAttribute,
}
const_assert_eq!(core::mem::size_of::<OpenSaveDataIn>(), 0x48);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ReadExtraDataBySpaceIdIn {
    pub save_data_space_id: u8,
    pub _pad: [u8; 7],
    pub save_id: u64,
}
const_assert_eq!(core::mem::size_of::<ReadExtraDataBySpaceIdIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct WriteExtraDataIn {
    pub save_data_space_id: u8,
    pub _pad: [u8; 7],
    pub save_id: u64,
}
const_assert_eq!(core::mem::size_of::<WriteExtraDataIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenSaveDataInfoReaderWithFilterIn {
    pub save_data_space_id: u8,
    pub pad: [u8; 7],
    pub filter: SaveDataFilter,
}
const_assert_eq!(
    core::mem::size_of::<OpenSaveDataInfoReaderWithFilterIn>(),
    0x50
);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenDataStorageByDataIdIn {
    pub storage_id: u8,
    pub _pad: [u8; 7],
    pub data_id: u64,
}
const_assert_eq!(core::mem::size_of::<OpenDataStorageByDataIdIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetRightsIdAndKeyGenOut {
    pub key_generation: u8,
    pub padding: [u8; 7],
    pub rights_id: RightsId,
}
const_assert_eq!(core::mem::size_of::<GetRightsIdAndKeyGenOut>(), 0x18);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ProgramIndexForAccessLogOut {
    pub index: u32,
    pub count: u32,
}
const_assert_eq!(core::mem::size_of::<ProgramIndexForAccessLogOut>(), 0x8);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct FsCreateFileIn {
    pub option: u32,
    pub _pad: u32,
    pub size: i64,
}
const_assert_eq!(core::mem::size_of::<FsCreateFileIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct FileReadIn {
    pub option: u32,
    pub pad: u32,
    pub offset: i64,
    pub read_size: u64,
}
const_assert_eq!(core::mem::size_of::<FileReadIn>(), 0x18);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct FileWriteIn {
    pub option: u32,
    pub pad: u32,
    pub offset: i64,
    pub write_size: u64,
}
const_assert_eq!(core::mem::size_of::<FileWriteIn>(), 0x18);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OperateRangeIn {
    pub op_id: u32,
    pub pad: u32,
    pub off: i64,
    pub len: i64,
}
const_assert_eq!(core::mem::size_of::<OperateRangeIn>(), 0x18);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct StorageReadWriteIn {
    pub offset: i64,
    pub size: u64,
}
const_assert_eq!(core::mem::size_of::<StorageReadWriteIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetDeviceCertIn {
    pub handle: GameCardHandle,
    pub buffer_size: i64,
}
const_assert_eq!(core::mem::size_of::<GetDeviceCertIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetAndClearStorageErrorInfoOut {
    pub error_info: StorageErrorInfo,
    pub log_size: i64,
}
const_assert_eq!(core::mem::size_of::<GetAndClearStorageErrorInfoOut>(), 0x18);
