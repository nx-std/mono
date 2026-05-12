//! Idle System (`idle:sys`) protocol constants.

use nx_sf::ServiceName;

/// Service name for the idle system interface.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("idle:sys");

/// Report that the user is active (resets the sleep counter).
pub const REPORT_USER_IS_ACTIVE: u32 = 5;
