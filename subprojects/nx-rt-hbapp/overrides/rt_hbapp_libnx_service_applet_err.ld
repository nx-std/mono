/*
 * Error applet (`error` library applet) API
 * Rust: ffi/libnx/applet_err.rs
 * libnx: applets/error.c
 *
 * Covers every `error*` entry point, not only the ported ones. libnx reaches
 * the applet through `g_appletILibraryAppletCreator`, which is `static` in
 * applet.c and cannot be aliased, so once `appletInitialize` is overridden no
 * `error*` function left to libnx has a session to use. Commands without an
 * implementation are aliased to stubs that panic naming the command.
 */

/* Application error */
EXTERN(__nx_rt_hbapp__libnx_error_application_create);
EXTERN(__nx_rt_hbapp__libnx_error_application_show);

/* Result and error code */
EXTERN(__nx_rt_hbapp__libnx_error_result_show);
EXTERN(__nx_rt_hbapp__libnx_error_code_show);
EXTERN(__nx_rt_hbapp__libnx_error_code_record_show);

/* Result backtrace */
EXTERN(__nx_rt_hbapp__libnx_error_result_backtrace_create);
EXTERN(__nx_rt_hbapp__libnx_error_result_backtrace_show);

/* System error */
EXTERN(__nx_rt_hbapp__libnx_error_system_create);
EXTERN(__nx_rt_hbapp__libnx_error_system_show);
EXTERN(__nx_rt_hbapp__libnx_error_system_set_context);

/* EULA */
EXTERN(__nx_rt_hbapp__libnx_error_eula_show);
EXTERN(__nx_rt_hbapp__libnx_error_system_update_eula_show);

/* Application error */
errorApplicationCreate = __nx_rt_hbapp__libnx_error_application_create;
errorApplicationShow = __nx_rt_hbapp__libnx_error_application_show;

/* Result and error code */
errorResultShow = __nx_rt_hbapp__libnx_error_result_show;
errorCodeShow = __nx_rt_hbapp__libnx_error_code_show;
errorCodeRecordShow = __nx_rt_hbapp__libnx_error_code_record_show;

/* Result backtrace */
errorResultBacktraceCreate = __nx_rt_hbapp__libnx_error_result_backtrace_create;
errorResultBacktraceShow = __nx_rt_hbapp__libnx_error_result_backtrace_show;

/* System error */
errorSystemCreate = __nx_rt_hbapp__libnx_error_system_create;
errorSystemShow = __nx_rt_hbapp__libnx_error_system_show;
errorSystemSetContext = __nx_rt_hbapp__libnx_error_system_set_context;

/* EULA */
errorEulaShow = __nx_rt_hbapp__libnx_error_eula_show;
errorSystemUpdateEulaShow = __nx_rt_hbapp__libnx_error_system_update_eula_show;
