/*
 * Controller applet (`controller` library applet) API
 * Rust: ffi/libnx/hid_la.rs
 * libnx: applets/hid_la.c
 *
 * Covers every `hidLa*` entry point. libnx reaches the applet through
 * `g_appletILibraryAppletCreator`, which is `static` in applet.c and cannot be
 * aliased, so once `appletInitialize` is overridden no `hidLa*` function left to
 * libnx has a session to use. All ten are ported.
 */

/* Argument construction */
EXTERN(__nx_rt_nro__libnx_hid_la_create_controller_support_arg);
EXTERN(__nx_rt_nro__libnx_hid_la_create_controller_firmware_update_arg);
EXTERN(__nx_rt_nro__libnx_hid_la_create_controller_key_remapping_arg);
EXTERN(__nx_rt_nro__libnx_hid_la_set_explain_text);

/* Argument construction */
hidLaCreateControllerSupportArg = __nx_rt_nro__libnx_hid_la_create_controller_support_arg;
hidLaCreateControllerFirmwareUpdateArg = __nx_rt_nro__libnx_hid_la_create_controller_firmware_update_arg;
hidLaCreateControllerKeyRemappingArg = __nx_rt_nro__libnx_hid_la_create_controller_key_remapping_arg;
hidLaSetExplainText = __nx_rt_nro__libnx_hid_la_set_explain_text;

/* Controller applet screens */
EXTERN(__nx_rt_nro__libnx_hid_la_show_controller_support);
EXTERN(__nx_rt_nro__libnx_hid_la_show_controller_strap_guide);
EXTERN(__nx_rt_nro__libnx_hid_la_show_controller_firmware_update);
EXTERN(__nx_rt_nro__libnx_hid_la_show_controller_support_for_system);
EXTERN(__nx_rt_nro__libnx_hid_la_show_controller_firmware_update_for_system);
EXTERN(__nx_rt_nro__libnx_hid_la_show_controller_key_remapping_for_system);

/* Controller applet screens */
hidLaShowControllerSupport = __nx_rt_nro__libnx_hid_la_show_controller_support;
hidLaShowControllerStrapGuide = __nx_rt_nro__libnx_hid_la_show_controller_strap_guide;
hidLaShowControllerFirmwareUpdate = __nx_rt_nro__libnx_hid_la_show_controller_firmware_update;
hidLaShowControllerSupportForSystem = __nx_rt_nro__libnx_hid_la_show_controller_support_for_system;
hidLaShowControllerFirmwareUpdateForSystem = __nx_rt_nro__libnx_hid_la_show_controller_firmware_update_for_system;
hidLaShowControllerKeyRemappingForSystem = __nx_rt_nro__libnx_hid_la_show_controller_key_remapping_for_system;
