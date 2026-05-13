//! Fatal service protocol constants.

use nx_sf::ServiceName;

/// Service name for the `fatal:u` interface.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("fatal:u");

/// Throws a fatal error with a policy (no CPU context).
pub const THROW_FATAL_WITH_POLICY: u32 = 1;

/// Throws a fatal error with a policy and CPU context.
pub const THROW_FATAL_WITH_CONTEXT: u32 = 2;
