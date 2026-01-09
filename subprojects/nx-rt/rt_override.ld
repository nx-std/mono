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
