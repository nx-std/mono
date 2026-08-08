/*
 * Cabinet applet (`cabinet` library applet) API
 * Rust: ffi/libnx/nfp.rs
 * libnx: applets/nfp_la.c
 *
 * Covers every `nfpLa*` entry point. libnx reaches the applet through
 * `g_appletILibraryAppletCreator`, which is `static` in applet.c and cannot be
 * aliased, so once `appletInitialize` is overridden no `nfpLa*` function left to
 * libnx has a session to use. All four are ported.
 */

/* Amiibo settings screens */
EXTERN(__nx_rt_nro__libnx_nfp_la_start_nickname_and_owner_settings);
EXTERN(__nx_rt_nro__libnx_nfp_la_start_game_data_eraser);
EXTERN(__nx_rt_nro__libnx_nfp_la_start_restorer);
EXTERN(__nx_rt_nro__libnx_nfp_la_start_formatter);

/* Amiibo settings screens */
nfpLaStartNicknameAndOwnerSettings = __nx_rt_nro__libnx_nfp_la_start_nickname_and_owner_settings;
nfpLaStartGameDataEraser = __nx_rt_nro__libnx_nfp_la_start_game_data_eraser;
nfpLaStartRestorer = __nx_rt_nro__libnx_nfp_la_start_restorer;
nfpLaStartFormatter = __nx_rt_nro__libnx_nfp_la_start_formatter;
