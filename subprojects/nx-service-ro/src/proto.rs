//! RO service protocol constants.

use nx_sf::ServiceName;

/// Service name for the legacy loader RO service (`ldr:ro`).
pub const LDR_RO_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ldr:ro");

/// Service name for the RO service variant 1 (`ro:1`, `[7.0.0+]`).
pub const RO1_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ro:1");

/// Service name for the RO debug/monitor service (`ro:dmnt`, `[3.0.0+]`).
pub const RO_DMNT_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ro:dmnt");

// IRoInterface commands (shared by ldr:ro and ro:1)
pub const LOAD_NRO: u32 = 0;
pub const UNLOAD_NRO: u32 = 1;
pub const LOAD_NRR: u32 = 2;
pub const UNLOAD_NRR: u32 = 3;
pub const INITIALIZE: u32 = 4;
pub const LOAD_NRR_EX: u32 = 10;

// IDebugMonitorInterface commands (ro:dmnt)
pub const GET_PROCESS_MODULE_INFO: u32 = 0;
