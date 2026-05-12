//! Multimedia (`mm:u`) protocol constants.

use nx_sf::ServiceName;

/// Service name for the multimedia service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("mm:u");

// Legacy commands (pre-2.0.0)

/// Initialise a request (legacy, pre-2.0.0). Input: module + unk + autoclear. Output: request ID.
pub const INITIALIZE_OLD: u32 = 0;

/// Finalise a request (legacy, pre-2.0.0). Input: module ID.
pub const FINALIZE_OLD: u32 = 1;

/// Set frequency and wait (legacy, pre-2.0.0). Input: module + freq + timeout.
pub const SET_AND_WAIT_OLD: u32 = 2;

/// Get current frequency (legacy, pre-2.0.0). Input: module ID. Output: freq.
pub const GET_OLD: u32 = 3;

// Commands (2.0.0+)

/// Initialise a request (2.0.0+). Input: module + unk + autoclear. Output: request ID.
pub const INITIALIZE: u32 = 4;

/// Finalise a request (2.0.0+). Input: request ID.
pub const FINALIZE: u32 = 5;

/// Set frequency and wait (2.0.0+). Input: request ID + freq + timeout.
pub const SET_AND_WAIT: u32 = 6;

/// Get current frequency (2.0.0+). Input: request ID. Output: freq.
pub const GET: u32 = 7;
