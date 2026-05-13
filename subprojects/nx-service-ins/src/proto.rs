//! INS service protocol constants.

use nx_sf::ServiceName;

/// Service name for the `ins:r` (request/read) interface.
pub const INSR_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ins:r");

/// Service name for the `ins:s` (send/write) interface.
pub const INSS_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ins:s");

// IInsRequest (ins:r) commands

/// Gets the last system tick an event was signaled at.
pub const GET_LAST_TICK: u32 = 0;

/// Gets a readable event by ID.
pub const GET_READABLE_EVENT: u32 = 1;

// InsSend (ins:s) commands

/// Gets a writable event by ID.
pub const GET_WRITABLE_EVENT: u32 = 0;
