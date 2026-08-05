//! The counters and failure records `fsp-srv` accumulates about itself.
//!
//! Every type here is read back by a get-and-clear command, so what it holds is
//! what has happened since the last time somebody asked, not the current state
//! of anything.

use static_assertions::const_assert_eq;

#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct StorageErrorInfo {
    pub num_activation_failures: u32,
    pub num_activation_error_corrections: u32,
    pub num_read_write_failures: u32,
    pub num_read_write_error_corrections: u32,
}
const_assert_eq!(core::mem::size_of::<StorageErrorInfo>(), 0x10);

#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct FatFatError {
    pub error: i32,
    pub extra_error: i32,
    pub drive_id: i32,
    pub name: [u8; 16],
    pub reserved: [u8; 4],
}
const_assert_eq!(core::mem::size_of::<FatFatError>(), 0x20);

#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct FatFatReportInfo1 {
    pub open_file_peak_count: u16,
    pub open_directory_peak_count: u16,
}
const_assert_eq!(core::mem::size_of::<FatFatReportInfo1>(), 0x4);

#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct FatFatReportInfo2 {
    pub open_unique_file_entry_peak_count: u16,
    pub open_unique_directory_entry_peak_count: u16,
}
const_assert_eq!(core::mem::size_of::<FatFatReportInfo2>(), 0x4);

#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct FatFatSafeInfo {
    pub result: u32,
    pub error_number: u32,
    pub safe_error_number: u32,
}
const_assert_eq!(core::mem::size_of::<FatFatSafeInfo>(), 0xC);

#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
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

#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
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

#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub(crate) struct GetAndClearStorageErrorInfoOut {
    pub error_info: StorageErrorInfo,
    pub log_size: i64,
}
const_assert_eq!(core::mem::size_of::<GetAndClearStorageErrorInfoOut>(), 0x18);
