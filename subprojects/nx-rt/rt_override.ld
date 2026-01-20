/* Linker script to override libnx runtime functions */

/* Heap initialization */
EXTERN(__nx_rt__initheap);
__libnx_initheap = __nx_rt__initheap;

/* Argv and nxlink symbols */
EXTERN(__nx_rt__argv_setup);
argvSetup = __nx_rt__argv_setup;

EXTERN(__nx_rt__system_argc);
EXTERN(__nx_rt__system_argv);
__system_argc = __nx_rt__system_argc;
__system_argv = __nx_rt__system_argv;

EXTERN(__nx_rt__nxlink_host);
__nxlink_host = __nx_rt__nxlink_host;

/* Environment symbols */
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

/* hosversion API */
EXTERN(__nx_rt__hosversion_get);
EXTERN(__nx_rt__hosversion_set);
EXTERN(__nx_rt__hosversion_is_atmosphere);

hosversionGet = __nx_rt__hosversion_get;
hosversionSet = __nx_rt__hosversion_set;
hosversionIsAtmosphere = __nx_rt__hosversion_is_atmosphere;

/* Service Manager API */
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

/* System Settings (set:sys) API */
EXTERN(__nx_rt__setsys_initialize);
EXTERN(__nx_rt__setsys_exit);
EXTERN(__nx_rt__setsys_get_service_session);
EXTERN(__nx_rt__setsys_get_firmware_version);

setsysInitialize = __nx_rt__setsys_initialize;
setsysExit = __nx_rt__setsys_exit;
setsysGetServiceSession = __nx_rt__setsys_get_service_session;
setsysGetFirmwareVersion = __nx_rt__setsys_get_firmware_version;

/* Application Performance Management (APM) API */
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

/* Applet Service API */
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

/* Human Input Device (HID) API */
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

/* Time Service API */
EXTERN(__nx_rt__time_initialize);
EXTERN(__nx_rt__time_exit);
EXTERN(__nx_rt__time_get_current_time);
EXTERN(__nx_rt__time_to_calendar_time_with_my_rule);

timeInitialize = __nx_rt__time_initialize;
timeExit = __nx_rt__time_exit;
timeGetCurrentTime = __nx_rt__time_get_current_time;
timeToCalendarTimeWithMyRule = __nx_rt__time_to_calendar_time_with_my_rule;

/* NV (NVIDIA Driver) Service API */
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
