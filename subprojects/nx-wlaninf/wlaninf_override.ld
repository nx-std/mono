/*
 * wlaninf.h ABI override — redirects libnx's `wlaninf*` symbols to the Rust
 * FFI in `nx-wlaninf` (`src/ffi.rs`).
 * libnx counterpart: `src/nx/source/services/wlaninf.c`.
 */

EXTERN(__nx_wlaninf__wlaninf_initialize);
EXTERN(__nx_wlaninf__wlaninf_exit);
EXTERN(__nx_wlaninf__wlaninf_get_service_session);
EXTERN(__nx_wlaninf__wlaninf_get_state);
EXTERN(__nx_wlaninf__wlaninf_get_rssi);

wlaninfInitialize        = __nx_wlaninf__wlaninf_initialize;
wlaninfExit              = __nx_wlaninf__wlaninf_exit;
wlaninfGetServiceSession = __nx_wlaninf__wlaninf_get_service_session;
wlaninfGetState          = __nx_wlaninf__wlaninf_get_state;
wlaninfGetRSSI           = __nx_wlaninf__wlaninf_get_rssi;
