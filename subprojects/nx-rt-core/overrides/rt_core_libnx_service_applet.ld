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
 * The `libapplet*` launcher surface is aliased here for the same reason: every
 * function in libnx's applets/libapplet.c dispatches through the same zeroed
 * g_appletILibraryAppletCreator once appletInitialize is overridden.
 *
 * Organization:
 *   - Rust FFI:  subprojects/nx-rt-core/src/ffi/libnx/applet.rs
 *                subprojects/nx-rt-core/src/ffi/libnx/libapplet.rs
 *   - libnx C:   subprojects/libnx/src/nx/source/services/applet.c
 *                subprojects/libnx/src/nx/source/applets/libapplet.c
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

/* Library applet launcher (applets/libapplet.c) */
EXTERN(__nx_rt_core__libnx_libapplet_args_create);
EXTERN(__nx_rt_core__libnx_libapplet_args_set_play_startup_sound);
EXTERN(__nx_rt_core__libnx_libapplet_create_write_storage);
EXTERN(__nx_rt_core__libnx_libapplet_read_storage);
EXTERN(__nx_rt_core__libnx_libapplet_args_push);
EXTERN(__nx_rt_core__libnx_libapplet_args_pop);
EXTERN(__nx_rt_core__libnx_libapplet_push_in_data);
EXTERN(__nx_rt_core__libnx_libapplet_pop_out_data);
EXTERN(__nx_rt_core__libnx_libapplet_set_jump_flag);
EXTERN(__nx_rt_core__libnx_libapplet_start);
EXTERN(__nx_rt_core__libnx_libapplet_launch);
EXTERN(__nx_rt_core__libnx_libapplet_request_home_menu);
EXTERN(__nx_rt_core__libnx_libapplet_request_jump_to_system_update);
EXTERN(__nx_rt_core__libnx_libapplet_request_to_launch_application);
EXTERN(__nx_rt_core__libnx_libapplet_request_jump_to_story);

libappletArgsCreate = __nx_rt_core__libnx_libapplet_args_create;
libappletArgsSetPlayStartupSound = __nx_rt_core__libnx_libapplet_args_set_play_startup_sound;
libappletCreateWriteStorage = __nx_rt_core__libnx_libapplet_create_write_storage;
libappletReadStorage = __nx_rt_core__libnx_libapplet_read_storage;
libappletArgsPush = __nx_rt_core__libnx_libapplet_args_push;
libappletArgsPop = __nx_rt_core__libnx_libapplet_args_pop;
libappletPushInData = __nx_rt_core__libnx_libapplet_push_in_data;
libappletPopOutData = __nx_rt_core__libnx_libapplet_pop_out_data;
libappletSetJumpFlag = __nx_rt_core__libnx_libapplet_set_jump_flag;
libappletStart = __nx_rt_core__libnx_libapplet_start;
libappletLaunch = __nx_rt_core__libnx_libapplet_launch;
libappletRequestHomeMenu = __nx_rt_core__libnx_libapplet_request_home_menu;
libappletRequestJumpToSystemUpdate = __nx_rt_core__libnx_libapplet_request_jump_to_system_update;
libappletRequestToLaunchApplication = __nx_rt_core__libnx_libapplet_request_to_launch_application;
libappletRequestJumpToStory = __nx_rt_core__libnx_libapplet_request_jump_to_story;
