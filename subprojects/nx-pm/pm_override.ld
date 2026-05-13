/*
 * pm.h ABI override — redirects libnx's `pm*` symbols to the Rust FFI in
 * `nx-pm` (`src/ffi/{bm,dmnt,info,shell}.rs`).
 * libnx counterpart: `src/nx/source/services/pm.c`.
 */

/* pm:bm */
EXTERN(__nx_pm__pmbm_initialize);
EXTERN(__nx_pm__pmbm_exit);
EXTERN(__nx_pm__pmbm_get_service_session);
EXTERN(__nx_pm__pmbm_get_boot_mode);
EXTERN(__nx_pm__pmbm_set_maintenance_boot);

pmbmInitialize         = __nx_pm__pmbm_initialize;
pmbmExit               = __nx_pm__pmbm_exit;
pmbmGetServiceSession  = __nx_pm__pmbm_get_service_session;
pmbmGetBootMode        = __nx_pm__pmbm_get_boot_mode;
pmbmSetMaintenanceBoot = __nx_pm__pmbm_set_maintenance_boot;

/* pm:dmnt */
EXTERN(__nx_pm__pmdmnt_initialize);
EXTERN(__nx_pm__pmdmnt_exit);
EXTERN(__nx_pm__pmdmnt_get_service_session);
EXTERN(__nx_pm__pmdmnt_get_jit_debug_process_id_list);
EXTERN(__nx_pm__pmdmnt_start_process);
EXTERN(__nx_pm__pmdmnt_get_process_id);
EXTERN(__nx_pm__pmdmnt_hook_to_create_process);
EXTERN(__nx_pm__pmdmnt_get_application_process_id);
EXTERN(__nx_pm__pmdmnt_hook_to_create_application_process);
EXTERN(__nx_pm__pmdmnt_clear_hook);
EXTERN(__nx_pm__pmdmnt_get_program_id);

pmdmntInitialize                     = __nx_pm__pmdmnt_initialize;
pmdmntExit                           = __nx_pm__pmdmnt_exit;
pmdmntGetServiceSession              = __nx_pm__pmdmnt_get_service_session;
pmdmntGetJitDebugProcessIdList       = __nx_pm__pmdmnt_get_jit_debug_process_id_list;
pmdmntStartProcess                   = __nx_pm__pmdmnt_start_process;
pmdmntGetProcessId                   = __nx_pm__pmdmnt_get_process_id;
pmdmntHookToCreateProcess            = __nx_pm__pmdmnt_hook_to_create_process;
pmdmntGetApplicationProcessId        = __nx_pm__pmdmnt_get_application_process_id;
pmdmntHookToCreateApplicationProcess = __nx_pm__pmdmnt_hook_to_create_application_process;
pmdmntClearHook                      = __nx_pm__pmdmnt_clear_hook;
pmdmntGetProgramId                   = __nx_pm__pmdmnt_get_program_id;

/* pm:info */
EXTERN(__nx_pm__pminfo_initialize);
EXTERN(__nx_pm__pminfo_exit);
EXTERN(__nx_pm__pminfo_get_service_session);
EXTERN(__nx_pm__pminfo_get_program_id);
EXTERN(__nx_pm__pminfo_get_applet_current_resource_limit_values);
EXTERN(__nx_pm__pminfo_get_applet_peak_resource_limit_values);

pminfoInitialize                          = __nx_pm__pminfo_initialize;
pminfoExit                                = __nx_pm__pminfo_exit;
pminfoGetServiceSession                   = __nx_pm__pminfo_get_service_session;
pminfoGetProgramId                        = __nx_pm__pminfo_get_program_id;
pminfoGetAppletCurrentResourceLimitValues = __nx_pm__pminfo_get_applet_current_resource_limit_values;
pminfoGetAppletPeakResourceLimitValues    = __nx_pm__pminfo_get_applet_peak_resource_limit_values;

/* pm:shell */
EXTERN(__nx_pm__pmshell_initialize);
EXTERN(__nx_pm__pmshell_exit);
EXTERN(__nx_pm__pmshell_get_service_session);
EXTERN(__nx_pm__pmshell_launch_program);
EXTERN(__nx_pm__pmshell_terminate_process);
EXTERN(__nx_pm__pmshell_terminate_program);
EXTERN(__nx_pm__pmshell_get_process_event_handle);
EXTERN(__nx_pm__pmshell_get_process_event_info);
EXTERN(__nx_pm__pmshell_cleanup_process);
EXTERN(__nx_pm__pmshell_clear_jit_debug_occured);
EXTERN(__nx_pm__pmshell_notify_boot_finished);
EXTERN(__nx_pm__pmshell_get_application_process_id_for_shell);
EXTERN(__nx_pm__pmshell_boost_system_memory_resource_limit);
EXTERN(__nx_pm__pmshell_boost_application_thread_resource_limit);
EXTERN(__nx_pm__pmshell_boost_system_thread_resource_limit);
EXTERN(__nx_pm__pmshell_get_process_id);

pmshellInitialize                       = __nx_pm__pmshell_initialize;
pmshellExit                             = __nx_pm__pmshell_exit;
pmshellGetServiceSession                = __nx_pm__pmshell_get_service_session;
pmshellLaunchProgram                    = __nx_pm__pmshell_launch_program;
pmshellTerminateProcess                 = __nx_pm__pmshell_terminate_process;
pmshellTerminateProgram                 = __nx_pm__pmshell_terminate_program;
pmshellGetProcessEventHandle            = __nx_pm__pmshell_get_process_event_handle;
pmshellGetProcessEventInfo              = __nx_pm__pmshell_get_process_event_info;
pmshellCleanupProcess                   = __nx_pm__pmshell_cleanup_process;
pmshellClearJitDebugOccured             = __nx_pm__pmshell_clear_jit_debug_occured;
pmshellNotifyBootFinished               = __nx_pm__pmshell_notify_boot_finished;
pmshellGetApplicationProcessIdForShell  = __nx_pm__pmshell_get_application_process_id_for_shell;
pmshellBoostSystemMemoryResourceLimit   = __nx_pm__pmshell_boost_system_memory_resource_limit;
pmshellBoostApplicationThreadResourceLimit = __nx_pm__pmshell_boost_application_thread_resource_limit;
pmshellBoostSystemThreadResourceLimit   = __nx_pm__pmshell_boost_system_thread_resource_limit;
pmshellGetProcessId                     = __nx_pm__pmshell_get_process_id;
