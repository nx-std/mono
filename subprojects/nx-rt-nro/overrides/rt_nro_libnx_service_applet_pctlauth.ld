/*
 * Parental Controls auth applet (`auth` library applet) API
 * Rust: ffi/libnx/pctlauth.rs
 * libnx: applets/pctlauth.c
 *
 * Covers every `pctlauth*` entry point. libnx reaches the applet through
 * `g_appletILibraryAppletCreator`, which is `static` in applet.c and cannot be
 * aliased, so once `appletInitialize` is overridden no `pctlauth*` function left
 * to libnx has a session to use. All five are ported.
 */

/* Parental Controls PIN screens */
EXTERN(__nx_rt_nro__libnx_pctlauth_show);
EXTERN(__nx_rt_nro__libnx_pctlauth_show_ex);
EXTERN(__nx_rt_nro__libnx_pctlauth_show_for_configuration);
EXTERN(__nx_rt_nro__libnx_pctlauth_register_passcode);
EXTERN(__nx_rt_nro__libnx_pctlauth_change_passcode);

/* Parental Controls PIN screens */
pctlauthShow = __nx_rt_nro__libnx_pctlauth_show;
pctlauthShowEx = __nx_rt_nro__libnx_pctlauth_show_ex;
pctlauthShowForConfiguration = __nx_rt_nro__libnx_pctlauth_show_for_configuration;
pctlauthRegisterPasscode = __nx_rt_nro__libnx_pctlauth_register_passcode;
pctlauthChangePasscode = __nx_rt_nro__libnx_pctlauth_change_passcode;
