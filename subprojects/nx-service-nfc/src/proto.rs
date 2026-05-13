//! NFC/NFP/Mifare service protocol constants.

use nx_sf::ServiceName;

// ---------------------------------------------------------------------------
// Service names
// ---------------------------------------------------------------------------

/// NFP user service name.
pub const NFP_USER_SERVICE_NAME: ServiceName = ServiceName::new_truncate("nfp:user");

/// NFP debug service name.
pub const NFP_DBG_SERVICE_NAME: ServiceName = ServiceName::new_truncate("nfp:dbg");

/// NFP system service name.
pub const NFP_SYS_SERVICE_NAME: ServiceName = ServiceName::new_truncate("nfp:sys");

/// NFC user service name.
pub const NFC_USER_SERVICE_NAME: ServiceName = ServiceName::new_truncate("nfc:user");

/// NFC system service name.
pub const NFC_SYS_SERVICE_NAME: ServiceName = ServiceName::new_truncate("nfc:sys");

/// NFC Mifare user service name.
pub const NFC_MF_SERVICE_NAME: ServiceName = ServiceName::new_truncate("nfc:mf:u");

// ---------------------------------------------------------------------------
// Root service command — shared by all three services
// ---------------------------------------------------------------------------

/// CreateInterface (cmd 0) — returns domain sub-object.
pub const CREATE_INTERFACE: u32 = 0;

// ---------------------------------------------------------------------------
// NFP interface commands (nfp:user / nfp:dbg / nfp:sys)
// ---------------------------------------------------------------------------

/// Initialize (sends PID + ARUID + MCU version buffer).
pub const NFP_INITIALIZE: u32 = 0;

/// Finalize.
pub const NFP_FINALIZE: u32 = 1;

/// ListDevices — returns device handle array + count.
pub const NFP_LIST_DEVICES: u32 = 2;

/// StartDetection (device handle in).
pub const NFP_START_DETECTION: u32 = 3;

/// StopDetection (device handle in).
pub const NFP_STOP_DETECTION: u32 = 4;

/// Mount (device handle + device type + mount target).
pub const NFP_MOUNT: u32 = 5;

/// Unmount (device handle in).
pub const NFP_UNMOUNT: u32 = 6;

/// OpenApplicationArea (device handle + app_id). Not for System.
pub const NFP_OPEN_APPLICATION_AREA: u32 = 7;

/// GetApplicationArea (device handle + out buffer + out size). Not for System.
pub const NFP_GET_APPLICATION_AREA: u32 = 8;

/// SetApplicationArea (device handle + in buffer). Not for System.
pub const NFP_SET_APPLICATION_AREA: u32 = 9;

/// Flush (device handle in).
pub const NFP_FLUSH: u32 = 10;

/// Restore (device handle in).
pub const NFP_RESTORE: u32 = 11;

/// CreateApplicationArea (device handle + app_id + in buffer). Not for System.
pub const NFP_CREATE_APPLICATION_AREA: u32 = 12;

/// GetTagInfo (device handle + out buffer).
pub const NFP_GET_TAG_INFO: u32 = 13;

/// GetRegisterInfo (device handle + out buffer).
pub const NFP_GET_REGISTER_INFO: u32 = 14;

/// GetCommonInfo (device handle + out buffer).
pub const NFP_GET_COMMON_INFO: u32 = 15;

/// GetModelInfo (device handle + out buffer).
pub const NFP_GET_MODEL_INFO: u32 = 16;

/// AttachActivateEvent (device handle + copy handle out).
pub const NFP_ATTACH_ACTIVATE_EVENT: u32 = 17;

/// AttachDeactivateEvent (device handle + copy handle out).
pub const NFP_ATTACH_DEACTIVATE_EVENT: u32 = 18;

/// GetState (out u32).
pub const NFP_GET_STATE: u32 = 19;

/// GetDeviceState (device handle + out u32).
pub const NFP_GET_DEVICE_STATE: u32 = 20;

/// GetNpadId (device handle + out u32).
pub const NFP_GET_NPAD_ID: u32 = 21;

/// GetApplicationAreaSize (device handle + out u32). Not for System.
pub const NFP_GET_APPLICATION_AREA_SIZE: u32 = 22;

/// AttachAvailabilityChangeEvent (copy handle out). [3.0.0+]
pub const NFP_ATTACH_AVAILABILITY_CHANGE_EVENT: u32 = 23;

/// RecreateApplicationArea (device handle + app_id + in buffer). [3.0.0+] Not for System.
pub const NFP_RECREATE_APPLICATION_AREA: u32 = 24;

// --- NFP system/debug-only commands ---

/// Format (device handle). Not for User.
pub const NFP_FORMAT: u32 = 100;

/// GetAdminInfo (device handle + out buffer). Not for User.
pub const NFP_GET_ADMIN_INFO: u32 = 101;

/// GetRegisterInfoPrivate (device handle + out buffer). Not for User.
pub const NFP_GET_REGISTER_INFO_PRIVATE: u32 = 102;

/// SetRegisterInfoPrivate (device handle + in buffer). Not for User.
pub const NFP_SET_REGISTER_INFO_PRIVATE: u32 = 103;

/// DeleteRegisterInfo (device handle). Not for User.
pub const NFP_DELETE_REGISTER_INFO: u32 = 104;

/// DeleteApplicationArea (device handle). Not for User.
pub const NFP_DELETE_APPLICATION_AREA: u32 = 105;

/// ExistsApplicationArea (device handle + out bool). Not for User.
pub const NFP_EXISTS_APPLICATION_AREA: u32 = 106;

// --- NFP debug-only commands ---

/// GetAll (device handle + out buffer). Debug only.
pub const NFP_GET_ALL: u32 = 200;

/// SetAll (device handle + in buffer). Debug only.
pub const NFP_SET_ALL: u32 = 201;

/// FlushDebug (device handle). Debug only.
pub const NFP_FLUSH_DEBUG: u32 = 202;

/// BreakTag (device handle + break type). Debug only.
pub const NFP_BREAK_TAG: u32 = 203;

/// ReadBackupData (device handle + out buffer + out size). Debug only.
pub const NFP_READ_BACKUP_DATA: u32 = 204;

/// WriteBackupData (device handle + in buffer). Debug only.
pub const NFP_WRITE_BACKUP_DATA: u32 = 205;

/// WriteNtf (device handle + write_type + in buffer). Debug only.
pub const NFP_WRITE_NTF: u32 = 206;

// ---------------------------------------------------------------------------
// NFC interface commands — pre-4.0.0 layout
// ---------------------------------------------------------------------------

/// Initialize (pre-4.0.0).
pub const NFC_INITIALIZE_LEGACY: u32 = 0;

/// Finalize (pre-4.0.0).
pub const NFC_FINALIZE_LEGACY: u32 = 1;

/// GetState (pre-4.0.0).
pub const NFC_GET_STATE_LEGACY: u32 = 2;

/// IsNfcEnabled (pre-4.0.0).
pub const NFC_IS_NFC_ENABLED_LEGACY: u32 = 3;

// ---------------------------------------------------------------------------
// NFC interface commands — 4.0.0+ layout
// ---------------------------------------------------------------------------

/// Initialize (4.0.0+).
pub const NFC_INITIALIZE: u32 = 400;

/// Finalize (4.0.0+).
pub const NFC_FINALIZE: u32 = 401;

/// GetState (4.0.0+).
pub const NFC_GET_STATE: u32 = 402;

/// IsNfcEnabled (4.0.0+).
pub const NFC_IS_NFC_ENABLED: u32 = 403;

/// ListDevices (4.0.0+).
pub const NFC_LIST_DEVICES: u32 = 404;

/// GetDeviceState (4.0.0+).
pub const NFC_GET_DEVICE_STATE: u32 = 405;

/// GetNpadId (4.0.0+).
pub const NFC_GET_NPAD_ID: u32 = 406;

/// AttachAvailabilityChangeEvent (4.0.0+).
pub const NFC_ATTACH_AVAILABILITY_CHANGE_EVENT: u32 = 407;

/// StartDetection (4.0.0+, device handle + protocol).
pub const NFC_START_DETECTION: u32 = 408;

/// StopDetection (4.0.0+).
pub const NFC_STOP_DETECTION: u32 = 409;

/// GetTagInfo (4.0.0+).
pub const NFC_GET_TAG_INFO: u32 = 410;

/// AttachActivateEvent (4.0.0+).
pub const NFC_ATTACH_ACTIVATE_EVENT: u32 = 411;

/// AttachDeactivateEvent (4.0.0+).
pub const NFC_ATTACH_DEACTIVATE_EVENT: u32 = 412;

// --- NFC Mifare commands (4.0.0+) ---

/// ReadMifare (4.0.0+).
pub const NFC_READ_MIFARE: u32 = 1000;

/// WriteMifare (4.0.0+).
pub const NFC_WRITE_MIFARE: u32 = 1001;

/// SendCommandByPassThrough (4.0.0+).
pub const NFC_SEND_COMMAND_BY_PASS_THROUGH: u32 = 1300;

/// KeepPassThroughSession (4.0.0+).
pub const NFC_KEEP_PASS_THROUGH_SESSION: u32 = 1301;

/// ReleasePassThroughSession (4.0.0+).
pub const NFC_RELEASE_PASS_THROUGH_SESSION: u32 = 1302;

// ---------------------------------------------------------------------------
// NFC Mifare interface commands (nfc:mf:u)
// ---------------------------------------------------------------------------

/// Initialize (MCU version + PID).
pub const MF_INITIALIZE: u32 = 0;

/// Finalize.
pub const MF_FINALIZE: u32 = 1;

/// ListDevices.
pub const MF_LIST_DEVICES: u32 = 2;

/// StartDetection.
pub const MF_START_DETECTION: u32 = 3;

/// StopDetection.
pub const MF_STOP_DETECTION: u32 = 4;

/// ReadMifare.
pub const MF_READ_MIFARE: u32 = 5;

/// WriteMifare.
pub const MF_WRITE_MIFARE: u32 = 6;

/// GetTagInfo.
pub const MF_GET_TAG_INFO: u32 = 7;

/// AttachActivateEvent.
pub const MF_ATTACH_ACTIVATE_EVENT: u32 = 8;

/// AttachDeactivateEvent.
pub const MF_ATTACH_DEACTIVATE_EVENT: u32 = 9;

/// GetState.
pub const MF_GET_STATE: u32 = 10;

/// GetDeviceState.
pub const MF_GET_DEVICE_STATE: u32 = 11;

/// GetNpadId.
pub const MF_GET_NPAD_ID: u32 = 12;

/// AttachAvailabilityChangeEvent.
pub const MF_ATTACH_AVAILABILITY_CHANGE_EVENT: u32 = 13;
