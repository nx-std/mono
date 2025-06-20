/* Static linker script for thread function symbols redirection */
/* Redirects libnx thread functions to nx-std-thread __nx_std_thread_* implementations */

/* newlib */
EXTERN(__nx_std_thread_newlib_sleep)
EXTERN(__nx_std_thread_newlib_usleep)
EXTERN(__nx_std_thread_newlib_nanosleep)
EXTERN(__nx_std_thread_newlib_sched_yield)

sleep = __nx_std_thread_newlib_sleep;
usleep = __nx_std_thread_newlib_usleep;
nanosleep = __nx_std_thread_newlib_nanosleep;
sched_yield = __nx_std_thread_newlib_sched_yield;
