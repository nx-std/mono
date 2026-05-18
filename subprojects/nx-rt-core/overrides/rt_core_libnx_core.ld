/* Linker script — kind-agnostic nx-rt-core overrides.
 *
 * Redirects the libnx C runtime + Service Manager symbols that every Switch
 * executable kind shares to the Rust FFI implementations in nx-rt-core.
 *
 * Kind-specific entry points (envSetup, argvSetup, __system_argc/argv,
 * __nxlink_host, __nx_applet_type) are deliberately NOT aliased here — each
 * output-kind entry crate (nx-rt-nro, nx-rt-nso, ...) owns those.
 *
 * Organization:
 *   - Rust FFI:  subprojects/nx-rt-core/src/ffi/libnx/*.rs
 *   - libnx C:   subprojects/libnx/src/nx/source/{runtime,services}/*.c
 */

/*
 * Heap Initialization
 * Rust: ffi/libnx/env.rs
 * libnx: runtime/init.c
 */
EXTERN(__nx_rt_core__libnx_initheap);
__libnx_initheap = __nx_rt_core__libnx_initheap;

/*
 * Main Thread TLS
 * Rust: ffi/libnx/env.rs
 * libnx: runtime/newlib.c
 */
EXTERN(__nx_rt_core__libnx_setup_main_thread_tls);
newlibSetup = __nx_rt_core__libnx_setup_main_thread_tls;

/*
 * Environment / Loader Configuration accessors
 * Rust: ffi/libnx/env.rs
 * libnx: runtime/env.c
 */
EXTERN(__nx_rt_core__libnx_env_get_loader_info);
EXTERN(__nx_rt_core__libnx_env_get_loader_info_size);
EXTERN(__nx_rt_core__libnx_env_get_main_thread_handle);
EXTERN(__nx_rt_core__libnx_env_is_nso);
EXTERN(__nx_rt_core__libnx_env_has_heap_override);
EXTERN(__nx_rt_core__libnx_env_get_heap_override_addr);
EXTERN(__nx_rt_core__libnx_env_get_heap_override_size);
EXTERN(__nx_rt_core__libnx_env_has_argv);
EXTERN(__nx_rt_core__libnx_env_get_argv);
EXTERN(__nx_rt_core__libnx_env_is_syscall_hinted);
EXTERN(__nx_rt_core__libnx_env_get_own_process_handle);
EXTERN(__nx_rt_core__libnx_env_get_exit_func_ptr);
EXTERN(__nx_rt_core__libnx_env_set_exit_func_ptr);
EXTERN(__nx_rt_core__libnx_env_set_next_load);
EXTERN(__nx_rt_core__libnx_env_has_next_load);
EXTERN(__nx_rt_core__libnx_env_get_last_load_result);
EXTERN(__nx_rt_core__libnx_env_has_random_seed);
EXTERN(__nx_rt_core__libnx_env_get_random_seed);
EXTERN(__nx_rt_core__libnx_env_get_user_id_storage);

envGetLoaderInfo      = __nx_rt_core__libnx_env_get_loader_info;
envGetLoaderInfoSize  = __nx_rt_core__libnx_env_get_loader_info_size;
envGetMainThreadHandle = __nx_rt_core__libnx_env_get_main_thread_handle;
envIsNso              = __nx_rt_core__libnx_env_is_nso;
envHasHeapOverride    = __nx_rt_core__libnx_env_has_heap_override;
envGetHeapOverrideAddr = __nx_rt_core__libnx_env_get_heap_override_addr;
envGetHeapOverrideSize = __nx_rt_core__libnx_env_get_heap_override_size;
envHasArgv            = __nx_rt_core__libnx_env_has_argv;
envGetArgv            = __nx_rt_core__libnx_env_get_argv;
envIsSyscallHinted    = __nx_rt_core__libnx_env_is_syscall_hinted;
envGetOwnProcessHandle = __nx_rt_core__libnx_env_get_own_process_handle;
envGetExitFuncPtr     = __nx_rt_core__libnx_env_get_exit_func_ptr;
envSetExitFuncPtr     = __nx_rt_core__libnx_env_set_exit_func_ptr;
envSetNextLoad        = __nx_rt_core__libnx_env_set_next_load;
envHasNextLoad        = __nx_rt_core__libnx_env_has_next_load;
envGetLastLoadResult  = __nx_rt_core__libnx_env_get_last_load_result;
envHasRandomSeed      = __nx_rt_core__libnx_env_has_random_seed;
envGetRandomSeed      = __nx_rt_core__libnx_env_get_random_seed;
envGetUserIdStorage   = __nx_rt_core__libnx_env_get_user_id_storage;

/*
 * HOS Version API
 * Rust: ffi/libnx/env.rs
 * libnx: runtime/hosversion.c
 */
EXTERN(__nx_rt_core__libnx_hosversion_get);
EXTERN(__nx_rt_core__libnx_hosversion_set);
EXTERN(__nx_rt_core__libnx_hosversion_is_atmosphere);

hosversionGet          = __nx_rt_core__libnx_hosversion_get;
hosversionSet          = __nx_rt_core__libnx_hosversion_set;
hosversionIsAtmosphere = __nx_rt_core__libnx_hosversion_is_atmosphere;

/*
 * Service Manager (sm) API — always linked; sm is the foundation for every other service.
 * Rust: ffi/libnx/sm.rs
 * libnx: services/sm.c
 */
EXTERN(__nx_rt_core__libnx_sm_initialize);
EXTERN(__nx_rt_core__libnx_sm_exit);
EXTERN(__nx_rt_core__libnx_sm_get_service_wrapper);
EXTERN(__nx_rt_core__libnx_sm_get_service_original);
EXTERN(__nx_rt_core__libnx_sm_get_service_override);
EXTERN(__nx_rt_core__libnx_sm_add_override_handle);
EXTERN(__nx_rt_core__libnx_sm_register_service);
EXTERN(__nx_rt_core__libnx_sm_register_service_cmif);
EXTERN(__nx_rt_core__libnx_sm_register_service_tipc);
EXTERN(__nx_rt_core__libnx_sm_unregister_service);
EXTERN(__nx_rt_core__libnx_sm_unregister_service_cmif);
EXTERN(__nx_rt_core__libnx_sm_unregister_service_tipc);
EXTERN(__nx_rt_core__libnx_sm_detach_client);
EXTERN(__nx_rt_core__libnx_sm_detach_client_cmif);
EXTERN(__nx_rt_core__libnx_sm_detach_client_tipc);
EXTERN(__nx_rt_core__libnx_sm_get_service_session);

smInitialize          = __nx_rt_core__libnx_sm_initialize;
smExit                = __nx_rt_core__libnx_sm_exit;
smGetServiceWrapper   = __nx_rt_core__libnx_sm_get_service_wrapper;
smGetServiceOriginal  = __nx_rt_core__libnx_sm_get_service_original;
smGetServiceOverride  = __nx_rt_core__libnx_sm_get_service_override;
smAddOverrideHandle   = __nx_rt_core__libnx_sm_add_override_handle;
smRegisterService     = __nx_rt_core__libnx_sm_register_service;
smRegisterServiceCmif = __nx_rt_core__libnx_sm_register_service_cmif;
smRegisterServiceTipc = __nx_rt_core__libnx_sm_register_service_tipc;
smUnregisterService   = __nx_rt_core__libnx_sm_unregister_service;
smUnregisterServiceCmif = __nx_rt_core__libnx_sm_unregister_service_cmif;
smUnregisterServiceTipc = __nx_rt_core__libnx_sm_unregister_service_tipc;
smDetachClient        = __nx_rt_core__libnx_sm_detach_client;
smDetachClientCmif    = __nx_rt_core__libnx_sm_detach_client_cmif;
smDetachClientTipc    = __nx_rt_core__libnx_sm_detach_client_tipc;
smGetServiceSession   = __nx_rt_core__libnx_sm_get_service_session;
