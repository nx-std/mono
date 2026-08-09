/*
 * Mii editor applet (`miiEdit` library applet) API
 * Rust: ffi/libnx/mii.rs
 * libnx: applets/mii_la.c
 *
 * Covers every `miiLa*` entry point. libnx reaches the applet through
 * `g_appletILibraryAppletCreator`, which is `static` in applet.c and cannot be
 * aliased, so once `appletInitialize` is overridden no `miiLa*` function left to
 * libnx has a session to use. All six are ported.
 */

/* Mii database screens */
EXTERN(__nx_rt_nro__libnx_mii_la_show_mii_edit);
EXTERN(__nx_rt_nro__libnx_mii_la_append_mii);
EXTERN(__nx_rt_nro__libnx_mii_la_append_mii_image);
EXTERN(__nx_rt_nro__libnx_mii_la_update_mii_image);

/* Mii database screens */
miiLaShowMiiEdit = __nx_rt_nro__libnx_mii_la_show_mii_edit;
miiLaAppendMii = __nx_rt_nro__libnx_mii_la_append_mii;
miiLaAppendMiiImage = __nx_rt_nro__libnx_mii_la_append_mii_image;
miiLaUpdateMiiImage = __nx_rt_nro__libnx_mii_la_update_mii_image;

/* Mii editing screens, [10.2.0+] */
EXTERN(__nx_rt_nro__libnx_mii_la_create_mii);
EXTERN(__nx_rt_nro__libnx_mii_la_edit_mii);

/* Mii editing screens, [10.2.0+] */
miiLaCreateMii = __nx_rt_nro__libnx_mii_la_create_mii;
miiLaEditMii = __nx_rt_nro__libnx_mii_la_edit_mii;
