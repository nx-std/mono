/* Static linker script for thread function symbols redirection */
/* Redirects libnx thread functions to __nx_sys_thread_* implementations */

/* Thread Local Storage (TLS) */
EXTERN(__nx_sys_thread_get_ptr);
EXTERN(__nx_sys_thread_get_thread_vars);
EXTERN(__nx_sys_thread_get_tls_start_offset);

armGetTls         = __nx_sys_thread_get_ptr;
getThreadVars     = __nx_sys_thread_get_thread_vars;
getTlsStartOffset = __nx_sys_thread_get_tls_start_offset;

/* Thread */
EXTERN(__nx_sys_thread_start)
EXTERN(__nx_sys_thread_pause)
EXTERN(__nx_sys_thread_resume)
EXTERN(__nx_sys_thread_dump_context)
EXTERN(__nx_sys_thread_get_cur_handle);
EXTERN(__nx_sys_thread_wait_for_exit)
EXTERN(__nx_sys_thread_get_self);
EXTERN(__nx_sys_thread_tls_get);
EXTERN(__nx_sys_thread_tls_set);

threadStart        = __nx_sys_thread_start;
threadPause        = __nx_sys_thread_pause;
threadResume       = __nx_sys_thread_resume;
threadDumpContext  = __nx_sys_thread_dump_context;
threadGetCurHandle = __nx_sys_thread_get_cur_handle;
threadWaitForExit  = __nx_sys_thread_wait_for_exit;
threadGetSelf      = __nx_sys_thread_get_self;
threadTlsGet       = __nx_sys_thread_tls_get;
threadTlsSet       = __nx_sys_thread_tls_set;

/* libc (newlib) */
EXTERN(__nx_sys_thread_newlib_sleep)
EXTERN(__nx_sys_thread_newlib_usleep)
EXTERN(__nx_sys_thread_newlib_nanosleep)
EXTERN(__nx_sys_thread_newlib_pthread_setspecific)
EXTERN(__nx_sys_thread_newlib_pthread_getspecific)
EXTERN(__nx_sys_thread_newlib_sched_yield)

sleep               = __nx_sys_thread_newlib_sleep;
usleep              = __nx_sys_thread_newlib_usleep;
nanosleep           = __nx_sys_thread_newlib_nanosleep;
pthread_setspecific = __nx_sys_thread_newlib_pthread_setspecific;
pthread_getspecific = __nx_sys_thread_newlib_pthread_getspecific;
sched_yield         = __nx_sys_thread_newlib_sched_yield;
