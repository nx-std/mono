/*
 * System Settings (set:sys) API
 * Rust: ffi/libnx/setsys.rs
 * libnx: services/set.c
 */
EXTERN(__nx_rt_nro__libnx_setsys_initialize);
EXTERN(__nx_rt_nro__libnx_setsys_exit);
EXTERN(__nx_rt_nro__libnx_setsys_get_service_session);
EXTERN(__nx_rt_nro__libnx_setsys_get_firmware_version);

setsysInitialize = __nx_rt_nro__libnx_setsys_initialize;
setsysExit = __nx_rt_nro__libnx_setsys_exit;
setsysGetServiceSession = __nx_rt_nro__libnx_setsys_get_service_session;
setsysGetFirmwareVersion = __nx_rt_nro__libnx_setsys_get_firmware_version;
