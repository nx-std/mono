//! Cryptographic Secure RNG (`csrng`) protocol constants.

use nx_sf::ServiceName;

/// Service name for the CSRNG interface.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("csrng");

/// Fills a buffer with cryptographically-secure random bytes.
pub const GET_RANDOM_BYTES: u32 = 0;
