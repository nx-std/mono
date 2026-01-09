//! SM protocol constants.

use core::ffi::CStr;

/// SM named port.
pub const SM_PORT_NAME: &CStr = c"sm:";

/// Register client (sends PID).
pub const REGISTER_CLIENT: u32 = 0;

/// Get service handle by name.
pub const GET_SERVICE_HANDLE: u32 = 1;

/// Register a new service.
pub const REGISTER_SERVICE: u32 = 2;

/// Unregister a service.
pub const UNREGISTER_SERVICE: u32 = 3;

/// Detach client session.
pub const DETACH_CLIENT: u32 = 4;
