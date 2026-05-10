/*
 * Application Performance Management (APM) API
 * Rust: ffi/apm.rs
 * libnx: services/apm.c
 */
EXTERN(__nx_rt__apm_initialize);
EXTERN(__nx_rt__apm_exit);
EXTERN(__nx_rt__apm_get_performance_mode);
EXTERN(__nx_rt__apm_set_performance_configuration);
EXTERN(__nx_rt__apm_get_performance_configuration);
EXTERN(__nx_rt__apm_get_service_session);
EXTERN(__nx_rt__apm_get_service_session_session);

apmInitialize = __nx_rt__apm_initialize;
apmExit = __nx_rt__apm_exit;
apmGetPerformanceMode = __nx_rt__apm_get_performance_mode;
apmSetPerformanceConfiguration = __nx_rt__apm_set_performance_configuration;
apmGetPerformanceConfiguration = __nx_rt__apm_get_performance_configuration;
apmGetServiceSession = __nx_rt__apm_get_service_session;
apmGetServiceSession_Session = __nx_rt__apm_get_service_session_session;
