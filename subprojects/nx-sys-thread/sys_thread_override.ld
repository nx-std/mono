/* Static linker script for thread function symbols redirection */
/* Redirects libnx thread.h and devkitPro/libsysbase newlib thread functions to nx-sys-thread __nx_sys_thread__* implementations */

/* libnx thread.h */
EXTERN(__nx_sys_thread__thread_create);
EXTERN(__nx_sys_thread__thread_start);
EXTERN(__nx_sys_thread__thread_exit);
EXTERN(__nx_sys_thread__thread_wait_for_exit);
EXTERN(__nx_sys_thread__thread_close);
EXTERN(__nx_sys_thread__thread_pause);
EXTERN(__nx_sys_thread__thread_resume);
EXTERN(__nx_sys_thread__thread_dump_context);
EXTERN(__nx_sys_thread__thread_get_self);
EXTERN(__nx_sys_thread__thread_get_cur_handle);
EXTERN(__nx_sys_thread__thread_tls_alloc);
EXTERN(__nx_sys_thread__thread_tls_get);
EXTERN(__nx_sys_thread__thread_tls_set);
EXTERN(__nx_sys_thread__thread_tls_free);
EXTERN(__nx_sys_thread__libnx_init_thread);

threadCreate        = __nx_sys_thread__thread_create;
threadStart         = __nx_sys_thread__thread_start;
threadExit          = __nx_sys_thread__thread_exit;
threadWaitForExit   = __nx_sys_thread__thread_wait_for_exit;
threadClose         = __nx_sys_thread__thread_close;
threadPause         = __nx_sys_thread__thread_pause;
threadResume        = __nx_sys_thread__thread_resume;
threadDumpContext   = __nx_sys_thread__thread_dump_context;
threadGetSelf       = __nx_sys_thread__thread_get_self;
threadGetCurHandle  = __nx_sys_thread__thread_get_cur_handle;
threadTlsAlloc      = __nx_sys_thread__thread_tls_alloc;
threadTlsGet        = __nx_sys_thread__thread_tls_get;
threadTlsSet        = __nx_sys_thread__thread_tls_set;
threadTlsFree       = __nx_sys_thread__thread_tls_free;
__libnx_init_thread = __nx_sys_thread__libnx_init_thread;

/* libsysbase (newlib) thread syscalls */
EXTERN(__nx_sys_thread__syscall_thread_self);
EXTERN(__nx_sys_thread__syscall_thread_exit);
EXTERN(__nx_sys_thread__syscall_thread_create);
EXTERN(__nx_sys_thread__syscall_thread_join);
EXTERN(__nx_sys_thread__syscall_thread_detach);
EXTERN(__nx_sys_thread__syscall_tls_create);
EXTERN(__nx_sys_thread__syscall_tls_set);
EXTERN(__nx_sys_thread__syscall_tls_get);
EXTERN(__nx_sys_thread__syscall_tls_delete);
EXTERN(__nx_sys_thread__syscall_nanosleep);
EXTERN(__nx_sys_thread__sched_yield);
EXTERN(__nx_sys_thread__sched_getcpu);

__syscall_thread_self   = __nx_sys_thread__syscall_thread_self;
__syscall_thread_exit   = __nx_sys_thread__syscall_thread_exit;
__syscall_thread_create = __nx_sys_thread__syscall_thread_create;
__syscall_thread_join   = __nx_sys_thread__syscall_thread_join;
__syscall_thread_detach = __nx_sys_thread__syscall_thread_detach;
__syscall_tls_create    = __nx_sys_thread__syscall_tls_create;
__syscall_tls_set       = __nx_sys_thread__syscall_tls_set;
__syscall_tls_get       = __nx_sys_thread__syscall_tls_get;
__syscall_tls_delete    = __nx_sys_thread__syscall_tls_delete;
__syscall_nanosleep     = __nx_sys_thread__syscall_nanosleep;
sched_yield             = __nx_sys_thread__sched_yield;
sched_getcpu            = __nx_sys_thread__sched_getcpu;
