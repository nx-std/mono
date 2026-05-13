//! PGL service protocol constants.

use nx_sf::ServiceName;

/// Service name for the PGL service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("pgl");

// Root service commands
pub const LAUNCH_PROGRAM: u32 = 0;
pub const TERMINATE_PROCESS: u32 = 1;
pub const LAUNCH_PROGRAM_FROM_HOST: u32 = 2;
pub const GET_HOST_CONTENT_META_INFO: u32 = 4;
pub const GET_APPLICATION_PROCESS_ID: u32 = 5;
pub const BOOST_SYSTEM_MEMORY_RESOURCE_LIMIT: u32 = 6;
pub const IS_PROCESS_TRACKED: u32 = 7;
pub const ENABLE_APPLICATION_CRASH_REPORT: u32 = 8;
pub const IS_APPLICATION_CRASH_REPORT_ENABLED: u32 = 9;
pub const ENABLE_APPLICATION_ALL_THREAD_DUMP_ON_CRASH: u32 = 10;
pub const TRIGGER_APPLICATION_SNAPSHOT_DUMPER: u32 = 12;
pub const GET_EVENT_OBSERVER: u32 = 20;

// EventObserver sub-object commands
pub const OBSERVER_GET_PROCESS_EVENT: u32 = 0;
pub const OBSERVER_GET_PROCESS_EVENT_INFO: u32 = 1;
