//! Error context service protocol constants.

use nx_sf::ServiceName;

/// Service name for the error context reader interface (`ectx:r`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("ectx:r");

/// Pulls error context associated with a descriptor and result code.
pub const PULL_CONTEXT: u32 = 1;
