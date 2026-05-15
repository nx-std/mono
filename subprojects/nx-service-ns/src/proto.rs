//! Service name constants and CMIF command IDs for all NS interfaces.

use nx_service_sm::ServiceName;

// ---------------------------------------------------------------------------
// Service names
// ---------------------------------------------------------------------------

pub const NS_AM_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ns:am");
pub const NS_AM2_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ns:am2");
pub const NS_EC_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ns:ec");
pub const NS_WEB_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ns:web");
pub const NS_RID_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ns:rid");
pub const NS_RT_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ns:rt");
pub const NS_RO_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ns:ro");

pub const NSVM_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ns:vm");
pub const NSDEV_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ns:dev");
pub const NSSU_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ns:su");

// ---------------------------------------------------------------------------
// Getter interface (obtain sub-interfaces from the ns:am2 session)
// ---------------------------------------------------------------------------

pub const GET_DYNAMIC_RIGHTS_INTERFACE: u32 = 7988;
pub const GET_READONLY_APPLICATION_CONTROL_DATA_INTERFACE: u32 = 7989;
pub const GET_READONLY_APPLICATION_RECORD_INTERFACE: u32 = 7991;
pub const GET_ECOMMERCE_INTERFACE: u32 = 7992;
pub const GET_APPLICATION_VERSION_INTERFACE: u32 = 7993;
pub const GET_FACTORY_RESET_INTERFACE: u32 = 7994;
pub const GET_ACCOUNT_PROXY_INTERFACE: u32 = 7995;
pub const GET_APPLICATION_MANAGER_INTERFACE: u32 = 7996;
pub const GET_DOWNLOAD_TASK_INTERFACE: u32 = 7997;
pub const GET_CONTENT_MANAGEMENT_INTERFACE: u32 = 7998;
pub const GET_DOCUMENT_INTERFACE: u32 = 7999;

// ---------------------------------------------------------------------------
// IReadOnlyApplicationControlDataInterface
// ---------------------------------------------------------------------------

pub const CTRL_DATA_GET_APPLICATION_CONTROL_DATA: u32 = 0;
pub const CTRL_DATA_GET_APPLICATION_DESIRED_LANGUAGE: u32 = 1;
pub const CTRL_DATA_GET_APPLICATION_CONTROL_DATA2: u32 = 6;
pub const CTRL_DATA_LIST_APPLICATION_TITLE2: u32 = 10;

// ---------------------------------------------------------------------------
// IECommerceInterface
// ---------------------------------------------------------------------------

pub const ECOMMERCE_REQUEST_LINK_DEVICE: u32 = 0;
pub const ECOMMERCE_REQUEST_SYNC_RIGHTS: u32 = 3;
pub const ECOMMERCE_REQUEST_UNLINK_DEVICE: u32 = 4;

// ---------------------------------------------------------------------------
// IFactoryResetInterface
// ---------------------------------------------------------------------------

pub const FACTORY_RESET_TO_FACTORY_SETTINGS: u32 = 100;
pub const FACTORY_RESET_WITHOUT_USER_SAVE_DATA: u32 = 101;
pub const FACTORY_RESET_FOR_REFURBISHMENT: u32 = 102;
pub const FACTORY_RESET_WITH_PLATFORM_REGION: u32 = 103;
pub const FACTORY_RESET_WITH_PLATFORM_REGION_AUTHENTICATION: u32 = 104;

// ---------------------------------------------------------------------------
// IApplicationManagerInterface
// ---------------------------------------------------------------------------

pub const APPMGR_LIST_APPLICATION_RECORD: u32 = 0;
pub const APPMGR_GET_APPLICATION_RECORD_UPDATE_SYSTEM_EVENT: u32 = 2;
pub const APPMGR_GET_APPLICATION_VIEW_DEPRECATED: u32 = 3;
pub const APPMGR_DELETE_APPLICATION_ENTITY: u32 = 4;
pub const APPMGR_DELETE_APPLICATION_COMPLETELY: u32 = 5;
pub const APPMGR_DELETE_REDUNDANT_APPLICATION_ENTITY: u32 = 7;
pub const APPMGR_IS_APPLICATION_ENTITY_MOVABLE: u32 = 8;
pub const APPMGR_MOVE_APPLICATION_ENTITY: u32 = 9;
pub const APPMGR_CALCULATE_APPLICATION_OCCUPIED_SIZE: u32 = 11;
pub const APPMGR_REQUEST_APPLICATION_UPDATE_INFO: u32 = 30;
pub const APPMGR_CANCEL_APPLICATION_DOWNLOAD: u32 = 32;
pub const APPMGR_RESUME_APPLICATION_DOWNLOAD: u32 = 33;
pub const APPMGR_CHECK_APPLICATION_LAUNCH_VERSION: u32 = 38;
pub const APPMGR_CALCULATE_APPLICATION_DOWNLOAD_REQUIRED_SIZE: u32 = 41;
pub const APPMGR_CLEANUP_SD_CARD: u32 = 42;
pub const APPMGR_CHECK_SD_CARD_MOUNT_STATUS: u32 = 43;
pub const APPMGR_GET_SD_CARD_MOUNT_STATUS_CHANGED_EVENT: u32 = 44;
pub const APPMGR_GET_TOTAL_SPACE_SIZE: u32 = 47;
pub const APPMGR_GET_FREE_SPACE_SIZE: u32 = 48;
pub const APPMGR_GET_GAME_CARD_UPDATE_DETECTION_EVENT: u32 = 52;
pub const APPMGR_DISABLE_APPLICATION_AUTO_DELETE: u32 = 53;
pub const APPMGR_ENABLE_APPLICATION_AUTO_DELETE: u32 = 54;
#[allow(dead_code)]
pub const APPMGR_GET_APPLICATION_DESIRED_LANGUAGE: u32 = 55;
pub const APPMGR_SET_APPLICATION_TERMINATE_RESULT: u32 = 56;
pub const APPMGR_CLEAR_APPLICATION_TERMINATE_RESULT: u32 = 57;
pub const APPMGR_GET_LAST_SD_CARD_MOUNT_UNEXPECTED_RESULT: u32 = 58;
pub const APPMGR_GET_REQUEST_SERVER_STOPPER: u32 = 65;
pub const APPMGR_CANCEL_APPLICATION_APPLY_DELTA: u32 = 67;
pub const APPMGR_RESUME_APPLICATION_APPLY_DELTA: u32 = 68;
pub const APPMGR_CALCULATE_APPLICATION_APPLY_DELTA_REQUIRED_SIZE: u32 = 69;
pub const APPMGR_RESUME_ALL: u32 = 70;
pub const APPMGR_GET_STORAGE_SIZE: u32 = 71;
pub const APPMGR_REQUEST_UPDATE_APPLICATION2: u32 = 85;
pub const APPMGR_DELETE_USER_SAVE_DATA_ALL: u32 = 201;
pub const APPMGR_DELETE_USER_SYSTEM_SAVE_DATA: u32 = 210;
pub const APPMGR_DELETE_SAVE_DATA: u32 = 211;
pub const APPMGR_UNREGISTER_NETWORK_SERVICE_ACCOUNT: u32 = 220;
pub const APPMGR_UNREGISTER_NETWORK_SERVICE_ACCOUNT_WITH_DELETION: u32 = 221;
#[allow(dead_code)]
pub const APPMGR_GET_APPLICATION_CONTROL_DATA: u32 = 400;
pub const APPMGR_REQUEST_DOWNLOAD_APPLICATION_CONTROL_DATA: u32 = 402;
pub const APPMGR_LIST_APPLICATION_TITLE: u32 = 407;
pub const APPMGR_LIST_APPLICATION_ICON: u32 = 408;
pub const APPMGR_REQUEST_CHECK_GAME_CARD_REGISTRATION: u32 = 502;
pub const APPMGR_REQUEST_GAME_CARD_REGISTRATION_GOLD_POINT: u32 = 503;
pub const APPMGR_REQUEST_REGISTER_GAME_CARD: u32 = 504;
pub const APPMGR_GET_GAME_CARD_MOUNT_FAILURE_EVENT: u32 = 505;
pub const APPMGR_IS_GAME_CARD_INSERTED: u32 = 506;
pub const APPMGR_ENSURE_GAME_CARD_ACCESS: u32 = 507;
pub const APPMGR_GET_LAST_GAME_CARD_MOUNT_FAILURE_RESULT: u32 = 508;
pub const APPMGR_LIST_APPLICATION_ID_ON_GAME_CARD: u32 = 509;
pub const APPMGR_COUNT_APPLICATION_CONTENT_META: u32 = 600;
pub const APPMGR_LIST_APPLICATION_CONTENT_META_STATUS: u32 = 601;
pub const APPMGR_IS_ANY_APPLICATION_RUNNING: u32 = 607;
pub const APPMGR_CLEAR_TASK_STATUS_LIST: u32 = 701;
pub const APPMGR_REQUEST_DOWNLOAD_TASK_LIST: u32 = 702;
pub const APPMGR_REQUEST_ENSURE_DOWNLOAD_TASK: u32 = 703;
pub const APPMGR_LIST_DOWNLOAD_TASK_STATUS: u32 = 704;
pub const APPMGR_REQUEST_DOWNLOAD_TASK_LIST_DATA: u32 = 705;
pub const APPMGR_TRY_COMMIT_CURRENT_APPLICATION_DOWNLOAD_TASK: u32 = 706;
pub const APPMGR_ENABLE_AUTO_COMMIT: u32 = 707;
pub const APPMGR_DISABLE_AUTO_COMMIT: u32 = 708;
pub const APPMGR_TRIGGER_DYNAMIC_COMMIT_EVENT: u32 = 709;
pub const APPMGR_TOUCH_APPLICATION: u32 = 904;
pub const APPMGR_IS_APPLICATION_UPDATE_REQUESTED: u32 = 906;
pub const APPMGR_WITHDRAW_APPLICATION_UPDATE_REQUEST: u32 = 907;
pub const APPMGR_REQUEST_VERIFY_APPLICATION_DEPRECATED: u32 = 1000;
pub const APPMGR_REQUEST_VERIFY_ADDON_CONTENTS_RIGHTS: u32 = 1002;
pub const APPMGR_REQUEST_VERIFY_APPLICATION: u32 = 1003;
pub const APPMGR_IS_ANY_APPLICATION_ENTITY_INSTALLED: u32 = 1300;
pub const APPMGR_CLEANUP_UNAVAILABLE_ADDON_CONTENTS: u32 = 1309;
pub const APPMGR_ESTIMATE_SIZE_TO_MOVE: u32 = 1311;
pub const APPMGR_FORMAT_SD_CARD: u32 = 1500;
pub const APPMGR_NEEDS_SYSTEM_UPDATE_TO_FORMAT_SD_CARD: u32 = 1501;
#[allow(dead_code)]
pub const APPMGR_GET_LAST_SD_CARD_FORMAT_UNEXPECTED_RESULT: u32 = 1502;
pub const APPMGR_GET_APPLICATION_VIEW: u32 = 1701;
pub const APPMGR_GET_APPLICATION_VIEW_DOWNLOAD_ERROR_CONTEXT: u32 = 1703;
pub const APPMGR_GET_APPLICATION_VIEW_WITH_PROMOTION_INFO: u32 = 1704;
pub const APPMGR_REQUEST_DOWNLOAD_APPLICATION_PREPURCHASED_RIGHTS: u32 = 1901;
pub const APPMGR_GET_SYSTEM_DELIVERY_INFO: u32 = 2000;
pub const APPMGR_SELECT_LATEST_SYSTEM_DELIVERY_INFO: u32 = 2001;
pub const APPMGR_VERIFY_DELIVERY_PROTOCOL_VERSION: u32 = 2002;
pub const APPMGR_GET_APPLICATION_DELIVERY_INFO: u32 = 2003;
pub const APPMGR_HAS_ALL_CONTENTS_TO_DELIVER: u32 = 2004;
pub const APPMGR_COMPARE_APPLICATION_DELIVERY_INFO: u32 = 2005;
pub const APPMGR_CAN_DELIVER_APPLICATION: u32 = 2006;
pub const APPMGR_LIST_CONTENT_META_KEY_TO_DELIVER_APPLICATION: u32 = 2007;
pub const APPMGR_NEEDS_SYSTEM_UPDATE_TO_DELIVER_APPLICATION: u32 = 2008;
pub const APPMGR_ESTIMATE_REQUIRED_SIZE: u32 = 2009;
pub const APPMGR_REQUEST_RECEIVE_APPLICATION: u32 = 2010;
pub const APPMGR_COMMIT_RECEIVE_APPLICATION: u32 = 2011;
pub const APPMGR_GET_RECEIVE_APPLICATION_PROGRESS: u32 = 2012;
pub const APPMGR_REQUEST_SEND_APPLICATION: u32 = 2013;
pub const APPMGR_GET_SEND_APPLICATION_PROGRESS: u32 = 2014;
pub const APPMGR_COMPARE_SYSTEM_DELIVERY_INFO: u32 = 2015;
pub const APPMGR_LIST_NOT_COMMITTED_CONTENT_META: u32 = 2016;
pub const APPMGR_GET_APPLICATION_DELIVERY_INFO_HASH: u32 = 2018;
pub const APPMGR_GET_APPLICATION_RIGHTS_ON_CLIENT: u32 = 2050;
pub const APPMGR_GET_APPLICATION_TERMINATE_RESULT: u32 = 2100;
pub const APPMGR_REQUEST_NO_DOWNLOAD_RIGHTS_ERROR_RESOLUTION: u32 = 2351;
pub const APPMGR_REQUEST_RESOLVE_NO_DOWNLOAD_RIGHTS_ERROR: u32 = 2352;
pub const APPMGR_GET_PROMOTION_INFO: u32 = 2400;

// ---------------------------------------------------------------------------
// IProgressMonitorForDeleteUserSaveDataAll (sub-object)
// ---------------------------------------------------------------------------

pub const PROGRESS_MONITOR_GET_SYSTEM_EVENT: u32 = 0;
pub const PROGRESS_MONITOR_IS_FINISHED: u32 = 1;
pub const PROGRESS_MONITOR_GET_RESULT: u32 = 2;
pub const PROGRESS_MONITOR_GET_PROGRESS: u32 = 10;

// ---------------------------------------------------------------------------
// IProgressAsyncResult (sub-object)
// ---------------------------------------------------------------------------

pub const PROGRESS_ASYNC_GET: u32 = 0;
pub const PROGRESS_ASYNC_CANCEL: u32 = 1;
pub const PROGRESS_ASYNC_GET_PROGRESS: u32 = 2;
pub const PROGRESS_ASYNC_GET_DETAIL_RESULT: u32 = 3;
pub const PROGRESS_ASYNC_GET_ERROR_CONTEXT: u32 = 4;

// ---------------------------------------------------------------------------
// ns:vm commands
// ---------------------------------------------------------------------------

pub const NSVM_NEEDS_UPDATE_VULNERABILITY: u32 = 1200;
pub const NSVM_GET_SAFE_SYSTEM_VERSION: u32 = 1202;

// ---------------------------------------------------------------------------
// ns:dev commands
// ---------------------------------------------------------------------------

pub const NSDEV_LAUNCH_PROGRAM: u32 = 0;
pub const NSDEV_TERMINATE_PROCESS: u32 = 1;
pub const NSDEV_TERMINATE_PROGRAM: u32 = 2;
pub const NSDEV_GET_SHELL_EVENT: u32 = 4;
pub const NSDEV_GET_SHELL_EVENT_INFO: u32 = 5;
pub const NSDEV_TERMINATE_APPLICATION: u32 = 6;
pub const NSDEV_PREPARE_LAUNCH_PROGRAM_FROM_HOST: u32 = 7;
pub const NSDEV_LAUNCH_APPLICATION_FOR_DEVELOP: u32 = 8;
pub const NSDEV_LAUNCH_APPLICATION_FROM_HOST: u32 = 8;
pub const NSDEV_LAUNCH_APPLICATION_WITH_STORAGE_ID_FOR_DEVELOP: u32 = 9;
pub const NSDEV_IS_SYSTEM_MEMORY_RESOURCE_LIMIT_BOOSTED: u32 = 10;
pub const NSDEV_GET_RUNNING_APPLICATION_PROCESS_ID_FOR_DEVELOP: u32 = 11;
pub const NSDEV_SET_CURRENT_APPLICATION_RIGHTS_ENVIRONMENT_CAN_BE_ACTIVE: u32 = 12;

// ---------------------------------------------------------------------------
// ns:su top-level commands
// ---------------------------------------------------------------------------

pub const NSSU_GET_BACKGROUND_NETWORK_UPDATE_STATE: u32 = 0;
pub const NSSU_OPEN_SYSTEM_UPDATE_CONTROL: u32 = 1;
pub const NSSU_NOTIFY_EXFAT_DRIVER_REQUIRED: u32 = 2;
pub const NSSU_CLEAR_EXFAT_DRIVER_STATUS_FOR_DEBUG: u32 = 3;
pub const NSSU_REQUEST_BACKGROUND_NETWORK_UPDATE: u32 = 4;
pub const NSSU_NOTIFY_BACKGROUND_NETWORK_UPDATE: u32 = 5;
pub const NSSU_NOTIFY_EXFAT_DRIVER_DOWNLOADED_FOR_DEBUG: u32 = 6;
pub const NSSU_GET_SYSTEM_UPDATE_NOTIFICATION_EVENT_FOR_CONTENT_DELIVERY: u32 = 9;
pub const NSSU_NOTIFY_SYSTEM_UPDATE_FOR_CONTENT_DELIVERY: u32 = 10;
pub const NSSU_PREPARE_SHUTDOWN: u32 = 11;
pub const NSSU_DESTROY_SYSTEM_UPDATE_TASK: u32 = 16;
pub const NSSU_REQUEST_SEND_SYSTEM_UPDATE: u32 = 17;
pub const NSSU_GET_SEND_SYSTEM_UPDATE_PROGRESS: u32 = 18;

// ---------------------------------------------------------------------------
// ISystemUpdateControl (sub-object on ns:su)
// ---------------------------------------------------------------------------

pub const NSSU_CTRL_HAS_DOWNLOADED: u32 = 0;
pub const NSSU_CTRL_REQUEST_CHECK_LATEST_UPDATE: u32 = 1;
pub const NSSU_CTRL_REQUEST_DOWNLOAD_LATEST_UPDATE: u32 = 2;
pub const NSSU_CTRL_GET_DOWNLOAD_PROGRESS: u32 = 3;
pub const NSSU_CTRL_APPLY_DOWNLOADED_UPDATE: u32 = 4;
pub const NSSU_CTRL_REQUEST_PREPARE_CARD_UPDATE: u32 = 5;
pub const NSSU_CTRL_GET_PREPARE_CARD_UPDATE_PROGRESS: u32 = 6;
pub const NSSU_CTRL_HAS_PREPARED_CARD_UPDATE: u32 = 7;
pub const NSSU_CTRL_APPLY_CARD_UPDATE: u32 = 8;
pub const NSSU_CTRL_GET_DOWNLOADED_EULA_DATA_SIZE: u32 = 9;
pub const NSSU_CTRL_GET_DOWNLOADED_EULA_DATA: u32 = 10;
pub const NSSU_CTRL_SETUP_CARD_UPDATE: u32 = 11;
pub const NSSU_CTRL_GET_PREPARED_CARD_UPDATE_EULA_DATA_SIZE: u32 = 12;
pub const NSSU_CTRL_GET_PREPARED_CARD_UPDATE_EULA_DATA: u32 = 13;
pub const NSSU_CTRL_SETUP_CARD_UPDATE_VIA_SYSTEM_UPDATER: u32 = 14;
pub const NSSU_CTRL_HAS_RECEIVED: u32 = 15;
pub const NSSU_CTRL_REQUEST_RECEIVE_SYSTEM_UPDATE: u32 = 16;
pub const NSSU_CTRL_GET_RECEIVE_PROGRESS: u32 = 17;
pub const NSSU_CTRL_APPLY_RECEIVED_UPDATE: u32 = 18;
pub const NSSU_CTRL_GET_RECEIVED_EULA_DATA_SIZE: u32 = 19;
pub const NSSU_CTRL_GET_RECEIVED_EULA_DATA: u32 = 20;
pub const NSSU_CTRL_SETUP_TO_RECEIVE_SYSTEM_UPDATE: u32 = 21;
pub const NSSU_CTRL_REQUEST_CHECK_LATEST_UPDATE_INCLUDES_REBOOTLESS_UPDATE: u32 = 22;
