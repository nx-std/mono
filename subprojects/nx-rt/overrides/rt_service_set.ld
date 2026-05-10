/*
 * System Settings (set:sys) API
 * Rust: ffi/setsys.rs
 * libnx: services/set.c
 */
EXTERN(__nx_rt__setsys_initialize);
EXTERN(__nx_rt__setsys_exit);
EXTERN(__nx_rt__setsys_get_service_session);
EXTERN(__nx_rt__setsys_get_firmware_version);

setsysInitialize = __nx_rt__setsys_initialize;
setsysExit = __nx_rt__setsys_exit;
setsysGetServiceSession = __nx_rt__setsys_get_service_session;
setsysGetFirmwareVersion = __nx_rt__setsys_get_firmware_version;
