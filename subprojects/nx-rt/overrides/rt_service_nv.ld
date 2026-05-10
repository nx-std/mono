/*
 * NV (NVIDIA Driver) Service API
 * Rust: ffi/nv.rs
 * libnx: services/nv.c
 */
EXTERN(__nx_rt__nv_initialize);
EXTERN(__nx_rt__nv_exit);
EXTERN(__nx_rt__nv_open);
EXTERN(__nx_rt__nv_ioctl);
EXTERN(__nx_rt__nv_ioctl2);
EXTERN(__nx_rt__nv_ioctl3);
EXTERN(__nx_rt__nv_close);
EXTERN(__nx_rt__nv_query_event);
EXTERN(__nx_rt__nv_convert_error);
EXTERN(__nx_rt__nv_get_service_session);

nvInitialize = __nx_rt__nv_initialize;
nvExit = __nx_rt__nv_exit;
nvOpen = __nx_rt__nv_open;
nvIoctl = __nx_rt__nv_ioctl;
nvIoctl2 = __nx_rt__nv_ioctl2;
nvIoctl3 = __nx_rt__nv_ioctl3;
nvClose = __nx_rt__nv_close;
nvQueryEvent = __nx_rt__nv_query_event;
nvConvertError = __nx_rt__nv_convert_error;
nvGetServiceSession = __nx_rt__nv_get_service_session;
