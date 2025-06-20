/* Static linker script for time function symbols redirection */
/* Redirects libnx functions to nx-time __nx_time_* implementations */

/* ARM Counter-Timer functions */
EXTERN(__nx_time_get_system_tick);
EXTERN(__nx_time_get_system_tick_freq);
EXTERN(__nx_time_ns_to_ticks);
EXTERN(__nx_time_ticks_to_ns);

armGetSystemTick = __nx_time_get_system_tick;
armGetSystemTickFreq = __nx_time_get_system_tick_freq;
armNsToTicks = __nx_time_ns_to_ticks;
armTicksToNs = __nx_time_ticks_to_ns;
