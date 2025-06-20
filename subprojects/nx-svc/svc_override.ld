/* Static linker script for SVC function symbols redirection */
/* Redirects libnx svc* functions to nx-svc __nx_svc_* implementations */

/* Memory management */
EXTERN(__nx_svc_set_heap_size);
EXTERN(__nx_svc_set_memory_permission);
EXTERN(__nx_svc_set_memory_attribute);
EXTERN(__nx_svc_map_memory);
EXTERN(__nx_svc_unmap_memory);
EXTERN(__nx_svc_query_memory);
EXTERN(__nx_svc_map_physical_memory);
EXTERN(__nx_svc_unmap_physical_memory);
EXTERN(__nx_svc_map_physical_memory_unsafe);
EXTERN(__nx_svc_unmap_physical_memory_unsafe);
EXTERN(__nx_svc_set_unsafe_limit);
EXTERN(__nx_svc_create_io_pool);
EXTERN(__nx_svc_create_io_region);
EXTERN(__nx_svc_map_io_region);
EXTERN(__nx_svc_unmap_io_region);
EXTERN(__nx_svc_map_insecure_physical_memory);
EXTERN(__nx_svc_unmap_insecure_physical_memory);
EXTERN(__nx_svc_set_process_memory_permission);
EXTERN(__nx_svc_map_process_memory);
EXTERN(__nx_svc_unmap_process_memory);
EXTERN(__nx_svc_query_process_memory);
EXTERN(__nx_svc_map_process_code_memory);
EXTERN(__nx_svc_unmap_process_code_memory);

svcSetHeapSize = __nx_svc_set_heap_size;
svcSetMemoryPermission = __nx_svc_set_memory_permission;
svcSetMemoryAttribute = __nx_svc_set_memory_attribute;
svcMapMemory = __nx_svc_map_memory;
svcUnmapMemory = __nx_svc_unmap_memory;
svcQueryMemory = __nx_svc_query_memory;
svcMapPhysicalMemory = __nx_svc_map_physical_memory;
svcUnmapPhysicalMemory = __nx_svc_unmap_physical_memory;
svcMapPhysicalMemoryUnsafe = __nx_svc_map_physical_memory_unsafe;
svcUnmapPhysicalMemoryUnsafe = __nx_svc_unmap_physical_memory_unsafe;
svcSetUnsafeLimit = __nx_svc_set_unsafe_limit;
svcCreateIoPool = __nx_svc_create_io_pool;
svcCreateIoRegion = __nx_svc_create_io_region;
svcMapIoRegion = __nx_svc_map_io_region;
svcUnmapIoRegion = __nx_svc_unmap_io_region;
svcMapInsecurePhysicalMemory = __nx_svc_map_insecure_physical_memory;
svcUnmapInsecurePhysicalMemory = __nx_svc_unmap_insecure_physical_memory;
svcSetProcessMemoryPermission = __nx_svc_set_process_memory_permission;
svcMapProcessMemory = __nx_svc_map_process_memory;
svcUnmapProcessMemory = __nx_svc_unmap_process_memory;
svcQueryProcessMemory = __nx_svc_query_process_memory;
svcMapProcessCodeMemory = __nx_svc_map_process_code_memory;
svcUnmapProcessCodeMemory = __nx_svc_unmap_process_code_memory;

/* Process and thread management */
EXTERN(__nx_svc_exit_process);
EXTERN(__nx_svc_create_thread);
EXTERN(__nx_svc_start_thread);
EXTERN(__nx_svc_exit_thread);
EXTERN(__nx_svc_sleep_thread);
EXTERN(__nx_svc_get_thread_priority);
EXTERN(__nx_svc_set_thread_priority);
EXTERN(__nx_svc_get_thread_core_mask);
EXTERN(__nx_svc_set_thread_core_mask);
EXTERN(__nx_svc_get_current_processor_number);
EXTERN(__nx_svc_get_process_id);
EXTERN(__nx_svc_get_thread_id);
EXTERN(__nx_svc_set_thread_activity);
EXTERN(__nx_svc_get_thread_context3);
EXTERN(__nx_svc_get_debug_future_thread_info);
EXTERN(__nx_svc_get_last_thread_info);
EXTERN(__nx_svc_set_process_activity);
EXTERN(__nx_svc_get_process_list);
EXTERN(__nx_svc_get_thread_list);
EXTERN(__nx_svc_create_process);
EXTERN(__nx_svc_start_process);
EXTERN(__nx_svc_terminate_process);
EXTERN(__nx_svc_get_process_info);

svcExitProcess = __nx_svc_exit_process;
svcCreateThread = __nx_svc_create_thread;
svcStartThread = __nx_svc_start_thread;
svcExitThread = __nx_svc_exit_thread;
svcSleepThread = __nx_svc_sleep_thread;
svcGetThreadPriority = __nx_svc_get_thread_priority;
svcSetThreadPriority = __nx_svc_set_thread_priority;
svcGetThreadCoreMask = __nx_svc_get_thread_core_mask;
svcSetThreadCoreMask = __nx_svc_set_thread_core_mask;
svcGetCurrentProcessorNumber = __nx_svc_get_current_processor_number;
svcGetProcessId = __nx_svc_get_process_id;
svcGetThreadId = __nx_svc_get_thread_id;
svcSetThreadActivity = __nx_svc_set_thread_activity;
svcGetThreadContext3 = __nx_svc_get_thread_context3;
svcGetDebugFutureThreadInfo = __nx_svc_get_debug_future_thread_info;
svcGetLastThreadInfo = __nx_svc_get_last_thread_info;
svcSetProcessActivity = __nx_svc_set_process_activity;
svcGetProcessList = __nx_svc_get_process_list;
svcGetThreadList = __nx_svc_get_thread_list;
svcCreateProcess = __nx_svc_create_process;
svcStartProcess = __nx_svc_start_process;
svcTerminateProcess = __nx_svc_terminate_process;
svcGetProcessInfo = __nx_svc_get_process_info;

/* Synchronization */
EXTERN(__nx_svc_signal_event);
EXTERN(__nx_svc_clear_event);
EXTERN(__nx_svc_reset_signal);
EXTERN(__nx_svc_wait_synchronization);
EXTERN(__nx_svc_cancel_synchronization);
EXTERN(__nx_svc_arbitrate_lock);
EXTERN(__nx_svc_arbitrate_unlock);
EXTERN(__nx_svc_wait_process_wide_key_atomic);
EXTERN(__nx_svc_signal_process_wide_key);
EXTERN(__nx_svc_wait_for_address);
EXTERN(__nx_svc_signal_to_address);
EXTERN(__nx_svc_synchronize_preemption_state);

svcSignalEvent = __nx_svc_signal_event;
svcClearEvent = __nx_svc_clear_event;
svcResetSignal = __nx_svc_reset_signal;
svcWaitSynchronization = __nx_svc_wait_synchronization;
svcCancelSynchronization = __nx_svc_cancel_synchronization;
svcArbitrateLock = __nx_svc_arbitrate_lock;
svcArbitrateUnlock = __nx_svc_arbitrate_unlock;
svcWaitProcessWideKeyAtomic = __nx_svc_wait_process_wide_key_atomic;
svcSignalProcessWideKey = __nx_svc_signal_process_wide_key;
svcWaitForAddress = __nx_svc_wait_for_address;
svcSignalToAddress = __nx_svc_signal_to_address;
svcSynchronizePreemptionState = __nx_svc_synchronize_preemption_state;

/* Inter-process memory sharing */
EXTERN(__nx_svc_map_shared_memory);
EXTERN(__nx_svc_unmap_shared_memory);
EXTERN(__nx_svc_create_transfer_memory);
EXTERN(__nx_svc_create_shared_memory);
EXTERN(__nx_svc_map_transfer_memory);
EXTERN(__nx_svc_unmap_transfer_memory);

svcMapSharedMemory = __nx_svc_map_shared_memory;
svcUnmapSharedMemory = __nx_svc_unmap_shared_memory;
svcCreateTransferMemory = __nx_svc_create_transfer_memory;
svcCreateSharedMemory = __nx_svc_create_shared_memory;
svcMapTransferMemory = __nx_svc_map_transfer_memory;
svcUnmapTransferMemory = __nx_svc_unmap_transfer_memory;

/* Miscellaneous */
EXTERN(__nx_svc_close_handle);
EXTERN(__nx_svc_get_system_tick);
EXTERN(__nx_svc_break);
EXTERN(__nx_svc_return_from_exception);
EXTERN(__nx_svc_get_info);
EXTERN(__nx_svc_get_system_info);

svcCloseHandle = __nx_svc_close_handle;
svcGetSystemTick = __nx_svc_get_system_tick;
svcBreak = __nx_svc_break;
svcReturnFromException = __nx_svc_return_from_exception;
svcGetInfo = __nx_svc_get_info;
svcGetSystemInfo = __nx_svc_get_system_info;

/* Cache Management */
EXTERN(__nx_svc_flush_entire_data_cache);
EXTERN(__nx_svc_flush_data_cache);
EXTERN(__nx_svc_invalidate_process_data_cache);
EXTERN(__nx_svc_store_process_data_cache);
EXTERN(__nx_svc_flush_process_data_cache);

svcFlushEntireDataCache = __nx_svc_flush_entire_data_cache;
svcFlushDataCache = __nx_svc_flush_data_cache;
svcInvalidateProcessDataCache = __nx_svc_invalidate_process_data_cache;
svcStoreProcessDataCache = __nx_svc_store_process_data_cache;
svcFlushProcessDataCache = __nx_svc_flush_process_data_cache;

/* Inter-process communication (IPC) */
EXTERN(__nx_svc_connect_to_named_port);
EXTERN(__nx_svc_send_sync_request_light);
EXTERN(__nx_svc_send_sync_request);
EXTERN(__nx_svc_send_sync_request_with_user_buffer);
EXTERN(__nx_svc_send_async_request_with_user_buffer);
EXTERN(__nx_svc_create_session);
EXTERN(__nx_svc_accept_session);
EXTERN(__nx_svc_reply_and_receive_light);
EXTERN(__nx_svc_reply_and_receive);
EXTERN(__nx_svc_reply_and_receive_with_user_buffer);
EXTERN(__nx_svc_create_event);
EXTERN(__nx_svc_create_port);
EXTERN(__nx_svc_manage_named_port);
EXTERN(__nx_svc_connect_to_port);

svcConnectToNamedPort = __nx_svc_connect_to_named_port;
svcSendSyncRequestLight = __nx_svc_send_sync_request_light;
svcSendSyncRequest = __nx_svc_send_sync_request;
svcSendSyncRequestWithUserBuffer = __nx_svc_send_sync_request_with_user_buffer;
svcSendAsyncRequestWithUserBuffer = __nx_svc_send_async_request_with_user_buffer;
svcCreateSession = __nx_svc_create_session;
svcAcceptSession = __nx_svc_accept_session;
svcReplyAndReceiveLight = __nx_svc_reply_and_receive_light;
svcReplyAndReceive = __nx_svc_reply_and_receive;
svcReplyAndReceiveWithUserBuffer = __nx_svc_reply_and_receive_with_user_buffer;
svcCreateEvent = __nx_svc_create_event;
svcCreatePort = __nx_svc_create_port;
svcManageNamedPort = __nx_svc_manage_named_port;
svcConnectToPort = __nx_svc_connect_to_port;

/* Resource Limit Management */
EXTERN(__nx_svc_get_resource_limit_limit_value);
EXTERN(__nx_svc_get_resource_limit_current_value);
EXTERN(__nx_svc_get_resource_limit_peak_value);
EXTERN(__nx_svc_create_resource_limit);
EXTERN(__nx_svc_set_resource_limit_limit_value);

svcGetResourceLimitLimitValue = __nx_svc_get_resource_limit_limit_value;
svcGetResourceLimitCurrentValue = __nx_svc_get_resource_limit_current_value;
svcGetResourceLimitPeakValue = __nx_svc_get_resource_limit_peak_value;
svcCreateResourceLimit = __nx_svc_create_resource_limit;
svcSetResourceLimitLimitValue = __nx_svc_set_resource_limit_limit_value;

/* Code memory / Just-in-time (JIT) compilation support */
EXTERN(__nx_svc_create_code_memory);
EXTERN(__nx_svc_control_code_memory);

svcCreateCodeMemory = __nx_svc_create_code_memory;
svcControlCodeMemory = __nx_svc_control_code_memory;

/* Power Management */
EXTERN(__nx_svc_sleep_system);

svcSleepSystem = __nx_svc_sleep_system;

/* Device memory-mapped I/O (MMIO) */
EXTERN(__nx_svc_read_write_register);
EXTERN(__nx_svc_create_interrupt_event);
EXTERN(__nx_svc_query_physical_address);
EXTERN(__nx_svc_query_memory_mapping);
EXTERN(__nx_svc_legacy_query_io_mapping);

svcReadWriteRegister = __nx_svc_read_write_register;
svcCreateInterruptEvent = __nx_svc_create_interrupt_event;
svcQueryPhysicalAddress = __nx_svc_query_physical_address;
svcQueryMemoryMapping = __nx_svc_query_memory_mapping;
svcLegacyQueryIoMapping = __nx_svc_legacy_query_io_mapping;

/* I/O memory management unit (IOMMU) */
EXTERN(__nx_svc_create_device_address_space);
EXTERN(__nx_svc_attach_device_address_space);
EXTERN(__nx_svc_detach_device_address_space);
EXTERN(__nx_svc_map_device_address_space_by_force);
EXTERN(__nx_svc_map_device_address_space_aligned);
EXTERN(__nx_svc_map_device_address_space);
EXTERN(__nx_svc_unmap_device_address_space);

svcCreateDeviceAddressSpace = __nx_svc_create_device_address_space;
svcAttachDeviceAddressSpace = __nx_svc_attach_device_address_space;
svcDetachDeviceAddressSpace = __nx_svc_detach_device_address_space;
svcMapDeviceAddressSpaceByForce = __nx_svc_map_device_address_space_by_force;
svcMapDeviceAddressSpaceAligned = __nx_svc_map_device_address_space_aligned;
svcMapDeviceAddressSpace = __nx_svc_map_device_address_space;
svcUnmapDeviceAddressSpace = __nx_svc_unmap_device_address_space;

/* Debugging */
EXTERN(__nx_svc_output_debug_string);
EXTERN(__nx_svc_dump_info);
EXTERN(__nx_svc_kernel_debug);
EXTERN(__nx_svc_change_kernel_trace_state);
EXTERN(__nx_svc_debug_active_process);
EXTERN(__nx_svc_break_debug_process);
EXTERN(__nx_svc_terminate_debug_process);
EXTERN(__nx_svc_get_debug_event);
EXTERN(__nx_svc_continue_debug_event);
EXTERN(__nx_svc_legacy_continue_debug_event);
EXTERN(__nx_svc_get_debug_thread_context);
EXTERN(__nx_svc_set_debug_thread_context);
EXTERN(__nx_svc_query_debug_process_memory);
EXTERN(__nx_svc_read_debug_process_memory);
EXTERN(__nx_svc_write_debug_process_memory);
EXTERN(__nx_svc_set_hardware_breakpoint);
EXTERN(__nx_svc_get_debug_thread_param);

svcOutputDebugString = __nx_svc_output_debug_string;
svcDumpInfo = __nx_svc_dump_info;
svcKernelDebug = __nx_svc_kernel_debug;
svcChangeKernelTraceState = __nx_svc_change_kernel_trace_state;
svcDebugActiveProcess = __nx_svc_debug_active_process;
svcBreakDebugProcess = __nx_svc_break_debug_process;
svcTerminateDebugProcess = __nx_svc_terminate_debug_process;
svcGetDebugEvent = __nx_svc_get_debug_event;
svcContinueDebugEvent = __nx_svc_continue_debug_event;
svcLegacyContinueDebugEvent = __nx_svc_legacy_continue_debug_event;
svcGetDebugThreadContext = __nx_svc_get_debug_thread_context;
svcSetDebugThreadContext = __nx_svc_set_debug_thread_context;
svcQueryDebugProcessMemory = __nx_svc_query_debug_process_memory;
svcReadDebugProcessMemory = __nx_svc_read_debug_process_memory;
svcWriteDebugProcessMemory = __nx_svc_write_debug_process_memory;
svcSetHardwareBreakPoint = __nx_svc_set_hardware_breakpoint;
svcGetDebugThreadParam = __nx_svc_get_debug_thread_param;

/* Secure monitor calls */
EXTERN(__nx_svc_call_secure_monitor);

svcCallSecureMonitor = __nx_svc_call_secure_monitor;
