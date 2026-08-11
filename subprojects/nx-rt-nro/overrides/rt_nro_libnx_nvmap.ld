/*
 * Memory-object (nvmap) API
 * Rust: ffi/libnx/nvmap.rs
 * The device descriptor is process-wide and sits beside the driver session
 * this crate owns, which is why these live here rather than in nx-nv.
 */
EXTERN(__nx_rt_nro__libnx_nvmap_init);
EXTERN(__nx_rt_nro__libnx_nvmap_exit);
EXTERN(__nx_rt_nro__libnx_nvmap_get_fd);
EXTERN(__nx_rt_nro__libnx_nvmap_create);
EXTERN(__nx_rt_nro__libnx_nvmap_load_remote);
EXTERN(__nx_rt_nro__libnx_nvmap_close);

nvMapInit = __nx_rt_nro__libnx_nvmap_init;
nvMapExit = __nx_rt_nro__libnx_nvmap_exit;
nvMapGetFd = __nx_rt_nro__libnx_nvmap_get_fd;
nvMapCreate = __nx_rt_nro__libnx_nvmap_create;
nvMapLoadRemote = __nx_rt_nro__libnx_nvmap_load_remote;
nvMapClose = __nx_rt_nro__libnx_nvmap_close;
