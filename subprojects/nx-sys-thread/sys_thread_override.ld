/* Static linker script for thread function symbols redirection */
/* Redirects libnx thread functions to __nx_sys_thread_* implementations */

/* Thread Local Storage (TLS) */
EXTERN(__nx_sys_thread__arm_get_tls);
EXTERN(__nx_sys_thread__get_thread_vars);
EXTERN(__nx_sys_thread__get_tls_start_offset);

armGetTls         = __nx_sys_thread__arm_get_tls;
getThreadVars     = __nx_sys_thread__get_thread_vars;
getTlsStartOffset = __nx_sys_thread__get_tls_start_offset;

/* Thread */
EXTERN(__nx_sys_thread__thread_start)
EXTERN(__nx_sys_thread__thread_pause)
EXTERN(__nx_sys_thread__thread_resume)
EXTERN(__nx_sys_thread__thread_dump_context)
EXTERN(__nx_sys_thread__thread_get_cur_handle);
EXTERN(__nx_sys_thread__thread_wait_for_exit)
EXTERN(__nx_sys_thread__thread_get_self);
EXTERN(__nx_sys_thread__thread_tls_get);
EXTERN(__nx_sys_thread__thread_tls_set);

threadStart        = __nx_sys_thread__thread_start;
threadPause        = __nx_sys_thread__thread_pause;
threadResume       = __nx_sys_thread__thread_resume;
threadDumpContext  = __nx_sys_thread__thread_dump_context;
threadGetCurHandle = __nx_sys_thread__thread_get_cur_handle;
threadWaitForExit  = __nx_sys_thread__thread_wait_for_exit;
threadGetSelf      = __nx_sys_thread__thread_get_self;
threadTlsGet       = __nx_sys_thread__thread_tls_get;
threadTlsSet       = __nx_sys_thread__thread_tls_set;

/* libc (newlib - libsysbase) */
EXTERN(__nx_sys_thread__libsysbase_syscall_thread_create)
EXTERN(__nx_sys_thread__libsysbase_syscall_thread_join)
EXTERN(__nx_sys_thread__libsysbase_syscall_thread_exit)
EXTERN(__nx_sys_thread__libsysbase_syscall_thread_self)
EXTERN(__nx_sys_thread__libsysbase_syscall_thread_detach)
EXTERN(__nx_sys_thread__libsysbase_syscall_tls_create)
EXTERN(__nx_sys_thread__libsysbase_syscall_tls_set)
EXTERN(__nx_sys_thread__libsysbase_syscall_tls_get)
EXTERN(__nx_sys_thread__libsysbase_syscall_tls_delete)

__syscall_thread_create = __nx_sys_thread__libsysbase_syscall_thread_create;
__syscall_thread_join   = __nx_sys_thread__libsysbase_syscall_thread_join;
__syscall_thread_exit   = __nx_sys_thread__libsysbase_syscall_thread_exit;
__syscall_thread_self   = __nx_sys_thread__libsysbase_syscall_thread_self;
__syscall_thread_detach = __nx_sys_thread__libsysbase_syscall_thread_detach;
__syscall_tls_create    = __nx_sys_thread__libsysbase_syscall_tls_create;
__syscall_tls_set       = __nx_sys_thread__libsysbase_syscall_tls_set;
__syscall_tls_get       = __nx_sys_thread__libsysbase_syscall_tls_get;
__syscall_tls_delete    = __nx_sys_thread__libsysbase_syscall_tls_delete;

EXTERN(__nx_sys_thread__libsysbase_sleep)
EXTERN(__nx_sys_thread__libsysbase_usleep)
EXTERN(__nx_sys_thread__libsysbase_nanosleep)
EXTERN(__nx_sys_thread__libsysbase_sched_yield)
EXTERN(__nx_sys_thread__libsysbase_sched_getcpu)

sleep        = __nx_sys_thread__libsysbase_sleep;
usleep       = __nx_sys_thread__libsysbase_usleep;
nanosleep    = __nx_sys_thread__libsysbase_nanosleep;
sched_yield  = __nx_sys_thread__libsysbase_sched_yield;
sched_getcpu = __nx_sys_thread__libsysbase_sched_getcpu;
