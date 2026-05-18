/* Linker script — kind-agnostic nx-rt-core applet accessor overrides.
 *
 * Redirects the role-independent libnx `applet*` accessor surface to the Rust
 * FFI implementations in nx-rt-core. Every Switch executable kind issues the
 * same calls against the runtime applet singleton, so this surface is shared.
 *
 * This fragment is gated by use_nx_service_applet on the consumer side: the
 * __nx_rt_core__libnx_applet_* symbols exist only when the `service-applet` Cargo
 * feature is active.
 *
 * The applet-type-sourcing entry points (appletInitialize, appletGetAppletType,
 * __nx_applet_type) are deliberately NOT aliased here — each output-kind entry
 * crate (nx-rt-nro, nx-rt-nso) owns those.
 *
 * Organization:
 *   - Rust FFI:  subprojects/nx-rt-core/src/ffi/libnx/applet.rs
 *   - libnx C:   subprojects/libnx/src/nx/source/services/applet.c
 */

EXTERN(__nx_rt_core__libnx_applet_exit);
EXTERN(__nx_rt_core__libnx_applet_get_operation_mode);
EXTERN(__nx_rt_core__libnx_applet_get_performance_mode);
EXTERN(__nx_rt_core__libnx_applet_get_focus_state);
EXTERN(__nx_rt_core__libnx_applet_get_message_event_handle);
EXTERN(__nx_rt_core__libnx_applet_get_message_event);
EXTERN(__nx_rt_core__libnx_applet_set_focus_handling_mode);
EXTERN(__nx_rt_core__libnx_applet_set_out_of_focus_suspending_enabled);
EXTERN(__nx_rt_core__libnx_applet_receive_message);
EXTERN(__nx_rt_core__libnx_applet_set_operation_mode_changed_notification);
EXTERN(__nx_rt_core__libnx_applet_set_performance_mode_changed_notification);
EXTERN(__nx_rt_core__libnx_applet_get_applet_resource_user_id);
EXTERN(__nx_rt_core__libnx_applet_acquire_foreground_rights);
EXTERN(__nx_rt_core__libnx_applet_create_managed_display_layer);

appletExit = __nx_rt_core__libnx_applet_exit;
appletGetOperationMode = __nx_rt_core__libnx_applet_get_operation_mode;
appletGetPerformanceMode = __nx_rt_core__libnx_applet_get_performance_mode;
appletGetFocusState = __nx_rt_core__libnx_applet_get_focus_state;
appletGetMessageEventHandle = __nx_rt_core__libnx_applet_get_message_event_handle;
appletGetMessageEvent = __nx_rt_core__libnx_applet_get_message_event;
appletSetFocusHandlingMode = __nx_rt_core__libnx_applet_set_focus_handling_mode;
appletSetOutOfFocusSuspendingEnabled = __nx_rt_core__libnx_applet_set_out_of_focus_suspending_enabled;
appletReceiveMessage = __nx_rt_core__libnx_applet_receive_message;
appletSetOperationModeChangedNotification = __nx_rt_core__libnx_applet_set_operation_mode_changed_notification;
appletSetPerformanceModeChangedNotification = __nx_rt_core__libnx_applet_set_performance_mode_changed_notification;
appletGetAppletResourceUserId = __nx_rt_core__libnx_applet_get_applet_resource_user_id;
appletAcquireForegroundRights = __nx_rt_core__libnx_applet_acquire_foreground_rights;
appletCreateManagedDisplayLayer = __nx_rt_core__libnx_applet_create_managed_display_layer;
