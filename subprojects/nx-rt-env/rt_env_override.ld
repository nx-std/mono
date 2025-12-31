/* Static linker script for runtime environment symbols redirection */
/* Redirects libnx env functions to nx-rt-env __nx_rt_env__* implementations */

EXTERN(__nx_rt_env__env_setup);
EXTERN(__nx_rt_env__env_get_loader_info);
EXTERN(__nx_rt_env__env_get_loader_info_size);
EXTERN(__nx_rt_env__env_get_main_thread_handle);
EXTERN(__nx_rt_env__env_is_nso);
EXTERN(__nx_rt_env__env_has_heap_override);
EXTERN(__nx_rt_env__env_get_heap_override_addr);
EXTERN(__nx_rt_env__env_get_heap_override_size);
EXTERN(__nx_rt_env__env_has_argv);
EXTERN(__nx_rt_env__env_get_argv);
EXTERN(__nx_rt_env__env_is_syscall_hinted);
EXTERN(__nx_rt_env__env_get_own_process_handle);
EXTERN(__nx_rt_env__env_get_exit_func_ptr);
EXTERN(__nx_rt_env__env_set_exit_func_ptr);
EXTERN(__nx_rt_env__env_set_next_load);
EXTERN(__nx_rt_env__env_has_next_load);
EXTERN(__nx_rt_env__env_get_last_load_result);
EXTERN(__nx_rt_env__env_has_random_seed);
EXTERN(__nx_rt_env__env_get_random_seed);
EXTERN(__nx_rt_env__env_get_user_id_storage);

envSetup = __nx_rt_env__env_setup;
envGetLoaderInfo = __nx_rt_env__env_get_loader_info;
envGetLoaderInfoSize = __nx_rt_env__env_get_loader_info_size;
envGetMainThreadHandle = __nx_rt_env__env_get_main_thread_handle;
envIsNso = __nx_rt_env__env_is_nso;
envHasHeapOverride = __nx_rt_env__env_has_heap_override;
envGetHeapOverrideAddr = __nx_rt_env__env_get_heap_override_addr;
envGetHeapOverrideSize = __nx_rt_env__env_get_heap_override_size;
envHasArgv = __nx_rt_env__env_has_argv;
envGetArgv = __nx_rt_env__env_get_argv;
envIsSyscallHinted = __nx_rt_env__env_is_syscall_hinted;
envGetOwnProcessHandle = __nx_rt_env__env_get_own_process_handle;
envGetExitFuncPtr = __nx_rt_env__env_get_exit_func_ptr;
envSetExitFuncPtr = __nx_rt_env__env_set_exit_func_ptr;
envSetNextLoad = __nx_rt_env__env_set_next_load;
envHasNextLoad = __nx_rt_env__env_has_next_load;
envGetLastLoadResult = __nx_rt_env__env_get_last_load_result;
envHasRandomSeed = __nx_rt_env__env_has_random_seed;
envGetRandomSeed = __nx_rt_env__env_get_random_seed;
envGetUserIdStorage = __nx_rt_env__env_get_user_id_storage;
