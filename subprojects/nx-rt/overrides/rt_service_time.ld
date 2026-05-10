/*
 * Time Service API
 * Rust: ffi/time.rs
 * libnx: services/time.c
 */
EXTERN(__nx_rt__time_initialize);
EXTERN(__nx_rt__time_exit);
EXTERN(__nx_rt__time_get_current_time);
EXTERN(__nx_rt__time_to_calendar_time_with_my_rule);

timeInitialize = __nx_rt__time_initialize;
timeExit = __nx_rt__time_exit;
timeGetCurrentTime = __nx_rt__time_get_current_time;
timeToCalendarTimeWithMyRule = __nx_rt__time_to_calendar_time_with_my_rule;
