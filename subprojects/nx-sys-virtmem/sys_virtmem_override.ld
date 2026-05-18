/* Static linker script for virtual memory manager symbols redirection */
/* Redirects libnx virtmem functions to nx-sys-virtmem __nx_sys_virtmem__virtmem_* implementations */

/* Virtual Memory Management (Virtmem) */
EXTERN(__nx_sys_virtmem__virtmem_lock);
EXTERN(__nx_sys_virtmem__virtmem_unlock);
EXTERN(__nx_sys_virtmem__virtmem_setup);
EXTERN(__nx_sys_virtmem__virtmem_find_aslr);
EXTERN(__nx_sys_virtmem__virtmem_find_stack);
EXTERN(__nx_sys_virtmem__virtmem_find_code_memory);
EXTERN(__nx_sys_virtmem__virtmem_add_reservation);
EXTERN(__nx_sys_virtmem__virtmem_remove_reservation);

virtmemLock              = __nx_sys_virtmem__virtmem_lock;
virtmemUnlock            = __nx_sys_virtmem__virtmem_unlock;
virtmemSetup             = __nx_sys_virtmem__virtmem_setup;
virtmemFindAslr          = __nx_sys_virtmem__virtmem_find_aslr;
virtmemFindStack         = __nx_sys_virtmem__virtmem_find_stack;
virtmemFindCodeMemory    = __nx_sys_virtmem__virtmem_find_code_memory;
virtmemAddReservation    = __nx_sys_virtmem__virtmem_add_reservation;
virtmemRemoveReservation = __nx_sys_virtmem__virtmem_remove_reservation;
