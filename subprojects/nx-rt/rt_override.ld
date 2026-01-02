/* Linker script to override libnx runtime functions */

EXTERN(__nx_rt__argv_setup);
argvSetup = __nx_rt__argv_setup;

EXTERN(__nx_rt__system_argc);
EXTERN(__nx_rt__system_argv);
__system_argc = __nx_rt__system_argc;
__system_argv = __nx_rt__system_argv;

EXTERN(__nx_rt__nxlink_host);
__nxlink_host = __nx_rt__nxlink_host;
