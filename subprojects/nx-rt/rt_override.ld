/* Linker script to override libnx runtime functions
 *
 * This script redirects libnx C function symbols to Rust FFI implementations
 * in the nx-rt crate, enabling gradual replacement of libnx with Rust code.
 *
 * Organization:
 *   - Rust FFI:  subprojects/nx-rt/src/ffi/*.rs
 *   - libnx C:   subprojects/libnx/src/nx/source/{runtime,services}/*.c
 */

/*
 * Heap Initialization
 * Rust: ffi/env.rs
 * libnx: runtime/init.c
 */
EXTERN(__nx_rt__initheap);
__libnx_initheap = __nx_rt__initheap;

/*
 * Argument Vector (argv)
 * Rust: ffi/argv.rs
 * libnx: runtime/argv.c
 */
EXTERN(__nx_rt__argv_setup);
argvSetup = __nx_rt__argv_setup;

EXTERN(__nx_rt__system_argc);
EXTERN(__nx_rt__system_argv);
__system_argc = __nx_rt__system_argc;
__system_argv = __nx_rt__system_argv;

/*
 * nxlink Host Address
 * Rust: ffi/env.rs
 * libnx: runtime/nxlink.c
 */
EXTERN(__nx_rt__nxlink_host);
__nxlink_host = __nx_rt__nxlink_host;

/*
 * Environment / Loader Configuration
 * Rust: ffi/env.rs
 * libnx: runtime/env.c, runtime/newlib.c, runtime/hosversion.c
 */
EXTERN(__nx_rt__env_setup);
EXTERN(__nx_rt__setup_main_thread_tls);
EXTERN(__nx_rt__env_get_loader_info);
EXTERN(__nx_rt__env_get_loader_info_size);
EXTERN(__nx_rt__env_get_main_thread_handle);
EXTERN(__nx_rt__env_is_nso);
EXTERN(__nx_rt__env_has_heap_override);
EXTERN(__nx_rt__env_get_heap_override_addr);
EXTERN(__nx_rt__env_get_heap_override_size);
EXTERN(__nx_rt__env_has_argv);
EXTERN(__nx_rt__env_get_argv);
EXTERN(__nx_rt__env_is_syscall_hinted);
EXTERN(__nx_rt__env_get_own_process_handle);
EXTERN(__nx_rt__env_get_exit_func_ptr);
EXTERN(__nx_rt__env_set_exit_func_ptr);
EXTERN(__nx_rt__env_set_next_load);
EXTERN(__nx_rt__env_has_next_load);
EXTERN(__nx_rt__env_get_last_load_result);
EXTERN(__nx_rt__env_has_random_seed);
EXTERN(__nx_rt__env_get_random_seed);
EXTERN(__nx_rt__env_get_user_id_storage);

envSetup = __nx_rt__env_setup;
newlibSetup = __nx_rt__setup_main_thread_tls;
envGetLoaderInfo = __nx_rt__env_get_loader_info;
envGetLoaderInfoSize = __nx_rt__env_get_loader_info_size;
envGetMainThreadHandle = __nx_rt__env_get_main_thread_handle;
envIsNso = __nx_rt__env_is_nso;
envHasHeapOverride = __nx_rt__env_has_heap_override;
envGetHeapOverrideAddr = __nx_rt__env_get_heap_override_addr;
envGetHeapOverrideSize = __nx_rt__env_get_heap_override_size;
envHasArgv = __nx_rt__env_has_argv;
envGetArgv = __nx_rt__env_get_argv;
envIsSyscallHinted = __nx_rt__env_is_syscall_hinted;
envGetOwnProcessHandle = __nx_rt__env_get_own_process_handle;
envGetExitFuncPtr = __nx_rt__env_get_exit_func_ptr;
envSetExitFuncPtr = __nx_rt__env_set_exit_func_ptr;
envSetNextLoad = __nx_rt__env_set_next_load;
envHasNextLoad = __nx_rt__env_has_next_load;
envGetLastLoadResult = __nx_rt__env_get_last_load_result;
envHasRandomSeed = __nx_rt__env_has_random_seed;
envGetRandomSeed = __nx_rt__env_get_random_seed;
envGetUserIdStorage = __nx_rt__env_get_user_id_storage;

/*
 * HOS Version API
 * Rust: ffi/env.rs
 * libnx: runtime/hosversion.c
 */
EXTERN(__nx_rt__hosversion_get);
EXTERN(__nx_rt__hosversion_set);
EXTERN(__nx_rt__hosversion_is_atmosphere);

hosversionGet = __nx_rt__hosversion_get;
hosversionSet = __nx_rt__hosversion_set;
hosversionIsAtmosphere = __nx_rt__hosversion_is_atmosphere;

/*
 * Service Manager (sm) API
 * Rust: ffi/sm.rs
 * libnx: services/sm.c
 */
EXTERN(__nx_rt__sm_initialize);
EXTERN(__nx_rt__sm_exit);
EXTERN(__nx_rt__sm_get_service_wrapper);
EXTERN(__nx_rt__sm_get_service_original);
EXTERN(__nx_rt__sm_get_service_override);
EXTERN(__nx_rt__sm_add_override_handle);
EXTERN(__nx_rt__sm_register_service);
EXTERN(__nx_rt__sm_register_service_cmif);
EXTERN(__nx_rt__sm_register_service_tipc);
EXTERN(__nx_rt__sm_unregister_service);
EXTERN(__nx_rt__sm_unregister_service_cmif);
EXTERN(__nx_rt__sm_unregister_service_tipc);
EXTERN(__nx_rt__sm_detach_client);
EXTERN(__nx_rt__sm_detach_client_cmif);
EXTERN(__nx_rt__sm_detach_client_tipc);
EXTERN(__nx_rt__sm_get_service_session);

smInitialize = __nx_rt__sm_initialize;
smExit = __nx_rt__sm_exit;
smGetServiceWrapper = __nx_rt__sm_get_service_wrapper;
smGetServiceOriginal = __nx_rt__sm_get_service_original;
smGetServiceOverride = __nx_rt__sm_get_service_override;
smAddOverrideHandle = __nx_rt__sm_add_override_handle;
smRegisterService = __nx_rt__sm_register_service;
smRegisterServiceCmif = __nx_rt__sm_register_service_cmif;
smRegisterServiceTipc = __nx_rt__sm_register_service_tipc;
smUnregisterService = __nx_rt__sm_unregister_service;
smUnregisterServiceCmif = __nx_rt__sm_unregister_service_cmif;
smUnregisterServiceTipc = __nx_rt__sm_unregister_service_tipc;
smDetachClient = __nx_rt__sm_detach_client;
smDetachClientCmif = __nx_rt__sm_detach_client_cmif;
smDetachClientTipc = __nx_rt__sm_detach_client_tipc;
smGetServiceSession = __nx_rt__sm_get_service_session;

/*
 * System Settings (set:sys) API
 * Rust: ffi/setsys.rs
 * libnx: services/set.c
 */
EXTERN(__nx_rt__setsys_initialize);
EXTERN(__nx_rt__setsys_exit);
EXTERN(__nx_rt__setsys_get_service_session);
EXTERN(__nx_rt__setsys_get_firmware_version);

setsysInitialize = __nx_rt__setsys_initialize;
setsysExit = __nx_rt__setsys_exit;
setsysGetServiceSession = __nx_rt__setsys_get_service_session;
setsysGetFirmwareVersion = __nx_rt__setsys_get_firmware_version;

/*
 * Application Performance Management (APM) API
 * Rust: ffi/apm.rs
 * libnx: services/apm.c
 */
EXTERN(__nx_rt__apm_initialize);
EXTERN(__nx_rt__apm_exit);
EXTERN(__nx_rt__apm_get_performance_mode);
EXTERN(__nx_rt__apm_set_performance_configuration);
EXTERN(__nx_rt__apm_get_performance_configuration);
EXTERN(__nx_rt__apm_get_service_session);
EXTERN(__nx_rt__apm_get_service_session_session);

apmInitialize = __nx_rt__apm_initialize;
apmExit = __nx_rt__apm_exit;
apmGetPerformanceMode = __nx_rt__apm_get_performance_mode;
apmSetPerformanceConfiguration = __nx_rt__apm_set_performance_configuration;
apmGetPerformanceConfiguration = __nx_rt__apm_get_performance_configuration;
apmGetServiceSession = __nx_rt__apm_get_service_session;
apmGetServiceSession_Session = __nx_rt__apm_get_service_session_session;

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

/*
 * Human Input Device (HID) API
 * Rust: ffi/hid.rs
 * libnx: services/hid.c
 */
EXTERN(__nx_rt__hid_initialize);
EXTERN(__nx_rt__hid_exit);
EXTERN(__nx_rt__hid_get_sharedmem_addr);
EXTERN(__nx_rt__hid_initialize_npad);
EXTERN(__nx_rt__hid_set_supported_npad_style_set);
EXTERN(__nx_rt__hid_set_supported_npad_id_type);
EXTERN(__nx_rt__hid_initialize_touch_screen);
EXTERN(__nx_rt__hid_initialize_mouse);
EXTERN(__nx_rt__hid_initialize_keyboard);
EXTERN(__nx_rt__hid_initialize_gesture);

hidInitialize = __nx_rt__hid_initialize;
hidExit = __nx_rt__hid_exit;
hidGetSharedmemAddr = __nx_rt__hid_get_sharedmem_addr;
hidInitializeNpad = __nx_rt__hid_initialize_npad;
hidSetSupportedNpadStyleSet = __nx_rt__hid_set_supported_npad_style_set;
hidSetSupportedNpadIdType = __nx_rt__hid_set_supported_npad_id_type;
hidInitializeTouchScreen = __nx_rt__hid_initialize_touch_screen;
hidInitializeMouse = __nx_rt__hid_initialize_mouse;
hidInitializeKeyboard = __nx_rt__hid_initialize_keyboard;
hidInitializeGesture = __nx_rt__hid_initialize_gesture;

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

/*
 * NV (NVIDIA Driver) Service API
 * Rust: ffi/nv.rs
 * libnx: services/nv.c
 */
EXTERN(__nx_rt__nv_initialize);
EXTERN(__nx_rt__nv_exit);
EXTERN(__nx_rt__nv_open);
EXTERN(__nx_rt__nv_ioctl);
EXTERN(__nx_rt__nv_ioctl2);
EXTERN(__nx_rt__nv_ioctl3);
EXTERN(__nx_rt__nv_close);
EXTERN(__nx_rt__nv_query_event);
EXTERN(__nx_rt__nv_convert_error);
EXTERN(__nx_rt__nv_get_service_session);

nvInitialize = __nx_rt__nv_initialize;
nvExit = __nx_rt__nv_exit;
nvOpen = __nx_rt__nv_open;
nvIoctl = __nx_rt__nv_ioctl;
nvIoctl2 = __nx_rt__nv_ioctl2;
nvIoctl3 = __nx_rt__nv_ioctl3;
nvClose = __nx_rt__nv_close;
nvQueryEvent = __nx_rt__nv_query_event;
nvConvertError = __nx_rt__nv_convert_error;
nvGetServiceSession = __nx_rt__nv_get_service_session;

/*
 * Visual Interface (VI) Service API
 * Rust: ffi/vi.rs
 * libnx: services/vi.c
 */
EXTERN(__nx_rt__vi_initialize);
EXTERN(__nx_rt__vi_exit);
EXTERN(__nx_rt__vi_get_session_application_display);
EXTERN(__nx_rt__vi_get_session_binder_relay);
EXTERN(__nx_rt__vi_get_session_system_display);
EXTERN(__nx_rt__vi_get_session_manager_display);
EXTERN(__nx_rt__vi_get_session_binder_indirect);
EXTERN(__nx_rt__vi_open_display);
EXTERN(__nx_rt__vi_close_display);
EXTERN(__nx_rt__vi_get_display_resolution);
EXTERN(__nx_rt__vi_get_display_logical_resolution);
EXTERN(__nx_rt__vi_set_display_magnification);
EXTERN(__nx_rt__vi_get_display_vsync_event);
EXTERN(__nx_rt__vi_set_display_power_state);
EXTERN(__nx_rt__vi_set_display_alpha);
EXTERN(__nx_rt__vi_get_z_order_count_min);
EXTERN(__nx_rt__vi_get_z_order_count_max);
EXTERN(__nx_rt__vi_create_layer);
EXTERN(__nx_rt__vi_create_managed_layer);
EXTERN(__nx_rt__vi_destroy_managed_layer);
EXTERN(__nx_rt__vi_close_layer);
EXTERN(__nx_rt__vi_set_layer_size);
EXTERN(__nx_rt__vi_set_layer_z);
EXTERN(__nx_rt__vi_set_layer_position);
EXTERN(__nx_rt__vi_set_layer_scaling_mode);
EXTERN(__nx_rt__vi_get_indirect_layer_image_map);
EXTERN(__nx_rt__vi_get_indirect_layer_image_required_memory_info);
EXTERN(__nx_rt__vi_set_content_visibility);
EXTERN(__nx_rt__vi_manager_prepare_fatal);
EXTERN(__nx_rt__vi_manager_show_fatal);
EXTERN(__nx_rt__vi_manager_draw_fatal_rectangle);
EXTERN(__nx_rt__vi_manager_draw_fatal_text32);

viInitialize = __nx_rt__vi_initialize;
viExit = __nx_rt__vi_exit;
viGetSession_IApplicationDisplayService = __nx_rt__vi_get_session_application_display;
viGetSession_IHOSBinderDriverRelay = __nx_rt__vi_get_session_binder_relay;
viGetSession_ISystemDisplayService = __nx_rt__vi_get_session_system_display;
viGetSession_IManagerDisplayService = __nx_rt__vi_get_session_manager_display;
viGetSession_IHOSBinderDriverIndirect = __nx_rt__vi_get_session_binder_indirect;
viOpenDisplay = __nx_rt__vi_open_display;
viCloseDisplay = __nx_rt__vi_close_display;
viGetDisplayResolution = __nx_rt__vi_get_display_resolution;
viGetDisplayLogicalResolution = __nx_rt__vi_get_display_logical_resolution;
viSetDisplayMagnification = __nx_rt__vi_set_display_magnification;
viGetDisplayVsyncEvent = __nx_rt__vi_get_display_vsync_event;
viSetDisplayPowerState = __nx_rt__vi_set_display_power_state;
viSetDisplayAlpha = __nx_rt__vi_set_display_alpha;
viGetZOrderCountMin = __nx_rt__vi_get_z_order_count_min;
viGetZOrderCountMax = __nx_rt__vi_get_z_order_count_max;
viCreateLayer = __nx_rt__vi_create_layer;
viCreateManagedLayer = __nx_rt__vi_create_managed_layer;
viDestroyManagedLayer = __nx_rt__vi_destroy_managed_layer;
viCloseLayer = __nx_rt__vi_close_layer;
viSetLayerSize = __nx_rt__vi_set_layer_size;
viSetLayerZ = __nx_rt__vi_set_layer_z;
viSetLayerPosition = __nx_rt__vi_set_layer_position;
viSetLayerScalingMode = __nx_rt__vi_set_layer_scaling_mode;
viGetIndirectLayerImageMap = __nx_rt__vi_get_indirect_layer_image_map;
viGetIndirectLayerImageRequiredMemoryInfo = __nx_rt__vi_get_indirect_layer_image_required_memory_info;
viSetContentVisibility = __nx_rt__vi_set_content_visibility;
viManagerPrepareFatal = __nx_rt__vi_manager_prepare_fatal;
viManagerShowFatal = __nx_rt__vi_manager_show_fatal;
viManagerDrawFatalRectangle = __nx_rt__vi_manager_draw_fatal_rectangle;
viManagerDrawFatalText32 = __nx_rt__vi_manager_draw_fatal_text32;
