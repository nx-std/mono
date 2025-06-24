/* Static linker script for thread function symbols redirection */
/* Redirects libnx thread functions to __nx_sys_thread_* implementations */

/* Thread Local Storage (TLS) */
EXTERN(__nx_sys_thread_get_ptr);

armGetTls = __nx_sys_thread_get_ptr;

/* Thread Variables */
EXTERN(__nx_sys_thread_get_thread_vars);
EXTERN(__nx_sys_thread_get_current_thread_handle);

getThreadVars = __nx_sys_thread_get_thread_vars;
threadGetCurHandle = __nx_sys_thread_get_current_thread_handle;

/* libc (newlib) */
EXTERN(__nx_std_thread_newlib_sleep)
EXTERN(__nx_std_thread_newlib_usleep)
EXTERN(__nx_std_thread_newlib_nanosleep)
EXTERN(__nx_std_thread_newlib_sched_yield)

sleep = __nx_std_thread_newlib_sleep;
usleep = __nx_std_thread_newlib_usleep;
nanosleep = __nx_std_thread_newlib_nanosleep;
sched_yield = __nx_std_thread_newlib_sched_yield;
