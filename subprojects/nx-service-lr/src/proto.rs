//! Location resolver service protocol constants.

use nx_sf::ServiceName;

pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("lr");

// ILocationResolverManager commands
pub const OPEN_LOCATION_RESOLVER: u32 = 0;
pub const OPEN_REGISTERED_LOCATION_RESOLVER: u32 = 1;

// ILocationResolver commands
pub const RESOLVE_PROGRAM_PATH: u32 = 0;
pub const REDIRECT_PROGRAM_PATH: u32 = 1;
pub const RESOLVE_APPLICATION_CONTROL_PATH: u32 = 2;
pub const RESOLVE_APPLICATION_HTML_DOCUMENT_PATH: u32 = 3;
pub const RESOLVE_DATA_PATH: u32 = 4;
pub const REDIRECT_APPLICATION_CONTROL_PATH: u32 = 5;
pub const REDIRECT_APPLICATION_HTML_DOCUMENT_PATH: u32 = 6;
pub const RESOLVE_APPLICATION_LEGAL_INFORMATION_PATH: u32 = 7;
pub const REDIRECT_APPLICATION_LEGAL_INFORMATION_PATH: u32 = 8;
pub const REFRESH: u32 = 9;
pub const ERASE_PROGRAM_REDIRECTION: u32 = 12;

// IRegisteredLocationResolver commands
pub const REG_RESOLVE_PROGRAM_PATH: u32 = 0;
