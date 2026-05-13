//! Process manager (`pm`) protocol constants.

use nx_sf::ServiceName;

/// Service name for `pm:bm` (boot mode).
pub const BM_SERVICE_NAME: ServiceName = ServiceName::new_truncate("pm:bm");

/// Service name for `pm:dmnt` (debug/monitor).
pub const DMNT_SERVICE_NAME: ServiceName = ServiceName::new_truncate("pm:dmnt");

/// Service name for `pm:info` (process info).
pub const INFO_SERVICE_NAME: ServiceName = ServiceName::new_truncate("pm:info");

/// Service name for `pm:shell` (shell).
pub const SHELL_SERVICE_NAME: ServiceName = ServiceName::new_truncate("pm:shell");

// pm:bm commands
pub const BM_GET_BOOT_MODE: u32 = 0;
pub const BM_SET_MAINTENANCE_BOOT: u32 = 1;

// pm:dmnt commands (5.0.0+)
pub const DMNT_GET_JIT_DEBUG_PROCESS_ID_LIST: u32 = 0;
pub const DMNT_START_PROCESS: u32 = 1;
pub const DMNT_GET_PROCESS_ID: u32 = 2;
pub const DMNT_HOOK_TO_CREATE_PROCESS: u32 = 3;
pub const DMNT_GET_APPLICATION_PROCESS_ID: u32 = 4;
pub const DMNT_HOOK_TO_CREATE_APPLICATION_PROCESS: u32 = 5;
pub const DMNT_CLEAR_HOOK: u32 = 6;
pub const DMNT_GET_PROGRAM_ID: u32 = 7;

// pm:dmnt commands (pre-5.0.0 legacy numbering)
pub const DMNT_GET_JIT_DEBUG_PROCESS_ID_LIST_LEGACY: u32 = 1;
pub const DMNT_START_PROCESS_LEGACY: u32 = 2;
pub const DMNT_GET_PROCESS_ID_LEGACY: u32 = 3;
pub const DMNT_HOOK_TO_CREATE_PROCESS_LEGACY: u32 = 4;
pub const DMNT_GET_APPLICATION_PROCESS_ID_LEGACY: u32 = 5;
pub const DMNT_HOOK_TO_CREATE_APPLICATION_PROCESS_LEGACY: u32 = 6;

// pm:info commands
pub const INFO_GET_PROGRAM_ID: u32 = 0;
pub const INFO_GET_APPLET_CURRENT_RESOURCE_LIMIT_VALUES: u32 = 1;
pub const INFO_GET_APPLET_PEAK_RESOURCE_LIMIT_VALUES: u32 = 2;

// pm:shell commands (same across versions for 0–4)
pub const SHELL_LAUNCH_PROGRAM: u32 = 0;
pub const SHELL_TERMINATE_PROCESS: u32 = 1;
pub const SHELL_TERMINATE_PROGRAM: u32 = 2;
pub const SHELL_GET_PROCESS_EVENT_HANDLE: u32 = 3;
pub const SHELL_GET_PROCESS_EVENT_INFO: u32 = 4;

// pm:shell commands (pre-5.0.0 only)
pub const SHELL_CLEANUP_PROCESS_LEGACY: u32 = 5;
pub const SHELL_CLEAR_JIT_DEBUG_OCCURRED_LEGACY: u32 = 6;

// pm:shell commands (pre-5.0.0 legacy numbering)
pub const SHELL_NOTIFY_BOOT_FINISHED_LEGACY: u32 = 7;
pub const SHELL_GET_APPLICATION_PROCESS_ID_FOR_SHELL_LEGACY: u32 = 8;
pub const SHELL_BOOST_SYSTEM_MEMORY_RESOURCE_LIMIT_LEGACY: u32 = 9;

// pm:shell commands (5.0.0+)
pub const SHELL_NOTIFY_BOOT_FINISHED: u32 = 5;
pub const SHELL_GET_APPLICATION_PROCESS_ID_FOR_SHELL: u32 = 6;
pub const SHELL_BOOST_SYSTEM_MEMORY_RESOURCE_LIMIT: u32 = 7;
pub const SHELL_BOOST_APPLICATION_THREAD_RESOURCE_LIMIT: u32 = 8;
pub const SHELL_BOOST_SYSTEM_THREAD_RESOURCE_LIMIT: u32 = 10;
pub const SHELL_GET_PROCESS_ID: u32 = 12;
