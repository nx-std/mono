/* Static linker script for TLS function symbols redirection */
/* Redirects libnx functions to nx-sys-thread-tls __nx_sys_thread_tls_* implementations */

/* TLS access functions */
EXTERN(__nx_sys_thread_tls__arm_get_tls);
EXTERN(__nx_sys_thread_tls__get_thread_vars);

armGetTls = __nx_sys_thread_tls__arm_get_tls;
getThreadVars = __nx_sys_thread_tls__get_thread_vars;
