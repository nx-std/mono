/* Static linker script for thread function symbols redirection */
/* Redirects libnx thread functions to nx-sys-mem __nx_sys_mem_* implementations */

/* Virtual Memory Management (VMM) */
EXTERN(__nx_sys_mem__virtmem_lock);
EXTERN(__nx_sys_mem__virtmem_unlock);
EXTERN(__nx_sys_mem__virtmem_find_aslr);
EXTERN(__nx_sys_mem__virtmem_find_stack);
EXTERN(__nx_sys_mem__virtmem_find_code_memory);
EXTERN(__nx_sys_mem__virtmem_add_reservation);
EXTERN(__nx_sys_mem__virtmem_remove_reservation);

virtmemLock              = __nx_sys_mem__virtmem_lock;
virtmemUnlock            = __nx_sys_mem__virtmem_unlock;
virtmemFindAslr          = __nx_sys_mem__virtmem_find_aslr;
virtmemFindStack         = __nx_sys_mem__virtmem_find_stack;
virtmemFindCodeMemory    = __nx_sys_mem__virtmem_find_code_memory;
virtmemAddReservation    = __nx_sys_mem__virtmem_add_reservation;
virtmemRemoveReservation = __nx_sys_mem__virtmem_remove_reservation;

EXTERN(__nx_sys_mem__virtmem_setup);
virtmemSetup = __nx_sys_mem__virtmem_setup;

/* Shared Memory (Shmem) */
EXTERN(__nx_sys_mem__shmem_create);
EXTERN(__nx_sys_mem__shmem_load_remote);
EXTERN(__nx_sys_mem__shmem_map);
EXTERN(__nx_sys_mem__shmem_unmap);
EXTERN(__nx_sys_mem__shmem_get_addr);
EXTERN(__nx_sys_mem__shmem_close);

shmemCreate     = __nx_sys_mem__shmem_create;
shmemLoadRemote = __nx_sys_mem__shmem_load_remote;
shmemMap        = __nx_sys_mem__shmem_map;
shmemUnmap      = __nx_sys_mem__shmem_unmap;
shmemGetAddr    = __nx_sys_mem__shmem_get_addr;
shmemClose      = __nx_sys_mem__shmem_close;

/* Transfer Memory (Tmem) */
EXTERN(__nx_sys_mem__tmem_create);
EXTERN(__nx_sys_mem__tmem_create_from_memory);
EXTERN(__nx_sys_mem__tmem_load_remote);
EXTERN(__nx_sys_mem__tmem_map);
EXTERN(__nx_sys_mem__tmem_unmap);
EXTERN(__nx_sys_mem__tmem_close_handle);
EXTERN(__nx_sys_mem__tmem_wait_for_permission);
EXTERN(__nx_sys_mem__tmem_close);

tmemCreate            = __nx_sys_mem__tmem_create;
tmemCreateFromMemory  = __nx_sys_mem__tmem_create_from_memory;
tmemLoadRemote        = __nx_sys_mem__tmem_load_remote;
tmemMap               = __nx_sys_mem__tmem_map;
tmemUnmap             = __nx_sys_mem__tmem_unmap;
tmemCloseHandle       = __nx_sys_mem__tmem_close_handle;
tmemWaitForPermission = __nx_sys_mem__tmem_wait_for_permission;
tmemClose             = __nx_sys_mem__tmem_close;
