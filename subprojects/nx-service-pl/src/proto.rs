//! PL service protocol constants.

use nx_sf::ServiceName;

/// Service name for `pl:u` (user interface).
pub const PLU_SERVICE_NAME: ServiceName = ServiceName::new_truncate("pl:u");

/// Service name for `pl:s` (system interface).
pub const PLS_SERVICE_NAME: ServiceName = ServiceName::new_truncate("pl:s");

/// Requests loading of a shared font into shared memory.
pub const REQUEST_LOAD: u32 = 0;

/// Gets the load state of a shared font (0 = loading, 1 = loaded).
pub const GET_LOAD_STATE: u32 = 1;

/// Gets the size of a shared font in bytes.
pub const GET_SIZE: u32 = 2;

/// Gets the byte offset of a shared font within shared memory.
pub const GET_SHARED_MEMORY_ADDRESS_OFFSET: u32 = 3;

/// Gets the shared memory native handle (copy handle).
pub const GET_SHARED_MEMORY_NATIVE_HANDLE: u32 = 4;

/// Gets shared font data for a given language code, returning types/offsets/sizes
/// via map-alias output buffers.
pub const GET_SHARED_FONT: u32 = 5;
