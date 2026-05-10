/*
 * Applet Service API
 * Rust: ffi/applet.rs (functions), ffi/env.rs (global variable)
 * libnx: services/applet.c
 */
EXTERN(__nx_rt__applet_type);
__nx_applet_type = __nx_rt__applet_type;

EXTERN(__nx_rt__applet_initialize);
EXTERN(__nx_rt__applet_exit);
EXTERN(__nx_rt__applet_get_applet_type);
EXTERN(__nx_rt__applet_get_operation_mode);
EXTERN(__nx_rt__applet_get_performance_mode);
EXTERN(__nx_rt__applet_get_focus_state);
EXTERN(__nx_rt__applet_get_message_event_handle);
EXTERN(__nx_rt__applet_set_focus_handling_mode);
EXTERN(__nx_rt__applet_set_out_of_focus_suspending_enabled);
EXTERN(__nx_rt__applet_get_applet_resource_user_id);
EXTERN(__nx_rt__applet_create_managed_display_layer);

appletInitialize = __nx_rt__applet_initialize;
appletExit = __nx_rt__applet_exit;
appletGetAppletType = __nx_rt__applet_get_applet_type;
appletGetOperationMode = __nx_rt__applet_get_operation_mode;
appletGetPerformanceMode = __nx_rt__applet_get_performance_mode;
appletGetFocusState = __nx_rt__applet_get_focus_state;
appletGetMessageEventHandle = __nx_rt__applet_get_message_event_handle;
appletSetFocusHandlingMode = __nx_rt__applet_set_focus_handling_mode;
appletSetOutOfFocusSuspendingEnabled = __nx_rt__applet_set_out_of_focus_suspending_enabled;
appletGetAppletResourceUserId = __nx_rt__applet_get_applet_resource_user_id;
appletCreateManagedDisplayLayer = __nx_rt__applet_create_managed_display_layer;
