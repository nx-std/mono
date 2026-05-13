//! News service protocol constants.

use nx_sf::ServiceName;

/// Service name for the news administrator service (`news:a`).
pub const SERVICE_NAME_ADMIN: ServiceName = ServiceName::new_truncate("news:a");

/// Service name for the news configuration service (`news:c`).
pub const SERVICE_NAME_CONFIG: ServiceName = ServiceName::new_truncate("news:c");

/// Service name for the news manager service (`news:m`).
pub const SERVICE_NAME_MANAGER: ServiceName = ServiceName::new_truncate("news:m");

/// Service name for the news post service (`news:p`).
pub const SERVICE_NAME_POST: ServiceName = ServiceName::new_truncate("news:p");

/// Service name for the news viewer service (`news:v`).
pub const SERVICE_NAME_VIEWER: ServiceName = ServiceName::new_truncate("news:v");

// INewsCreator commands (2.0.0+)
pub const CREATE_NEWS_SERVICE: u32 = 0;
pub const CREATE_NEWLY_ARRIVED_EVENT_HOLDER: u32 = 1;
pub const CREATE_NEWS_DATA_SERVICE: u32 = 2;
pub const CREATE_NEWS_DATABASE_SERVICE: u32 = 3;
pub const CREATE_OVERWRITE_EVENT_HOLDER: u32 = 4;

// INewsService commands (used directly pre-2.0.0 or via created service 2.0.0+)
pub const POST_LOCAL_NEWS: u32 = 10100;
pub const SET_PASSPHRASE: u32 = 20100;
pub const GET_SUBSCRIPTION_STATUS: u32 = 30100;
pub const GET_TOPIC_LIST: u32 = 30101;
pub const GET_SAVEDATA_USAGE: u32 = 30110;
pub const IS_SYSTEM_UPDATE_REQUIRED: u32 = 30200;
pub const GET_DATABASE_VERSION: u32 = 30210;
pub const REQUEST_IMMEDIATE_RECEPTION: u32 = 30300;
pub const SET_SUBSCRIPTION_STATUS: u32 = 40100;
pub const CLEAR_STORAGE: u32 = 40200;
pub const CLEAR_SUBSCRIPTION_STATUS_ALL: u32 = 40201;
pub const GET_NEWS_DATABASE_DUMP: u32 = 90100;

// INewsService commands (pre-2.0.0 sub-object creation)
pub const CREATE_NEWLY_ARRIVED_EVENT_HOLDER_LEGACY: u32 = 30900;
pub const CREATE_NEWS_DATA_SERVICE_LEGACY: u32 = 30901;
pub const CREATE_NEWS_DATABASE_SERVICE_LEGACY: u32 = 30902;

// INewsNewlyArrivedEventHolder commands
pub const EVENT_HOLDER_GET: u32 = 0;

// INewsDataService commands
pub const DATA_OPEN: u32 = 0;
pub const DATA_OPEN_WITH_RECORD_V1: u32 = 1;
pub const DATA_READ: u32 = 2;
pub const DATA_GET_SIZE: u32 = 3;
pub const DATA_OPEN_WITH_RECORD: u32 = 1001;

// INewsDatabaseService commands
pub const DATABASE_GET_LIST_V1: u32 = 0;
pub const DATABASE_COUNT: u32 = 1;
pub const DATABASE_GET_LIST: u32 = 1000;
