/*
 * Time Service API
 * Rust: ffi/libnx/time.rs
 * libnx: services/time.c
 */
EXTERN(__nx_rt_nro__libnx_time_initialize);
EXTERN(__nx_rt_nro__libnx_time_exit);
EXTERN(__nx_rt_nro__libnx_time_get_current_time);
EXTERN(__nx_rt_nro__libnx_time_to_calendar_time_with_my_rule);

timeInitialize = __nx_rt_nro__libnx_time_initialize;
timeExit = __nx_rt_nro__libnx_time_exit;
timeGetCurrentTime = __nx_rt_nro__libnx_time_get_current_time;
timeToCalendarTimeWithMyRule = __nx_rt_nro__libnx_time_to_calendar_time_with_my_rule;

/*
 * Startup wall-clock anchoring
 * Rust: ffi/libnx/time.rs
 * libnx: runtime/newlib.c
 */
EXTERN(__nx_rt_nro__libnx_init_time);

__libnx_init_time = __nx_rt_nro__libnx_init_time;
