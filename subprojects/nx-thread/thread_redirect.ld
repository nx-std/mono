/* Static linker script for thread function symbols redirection */
/* Redirects libnx thread functions to nx-thread __nx_thread_* implementations */

/* Virtual Memory Management */
EXTERN(__nx_thread_virtmem_lock);
EXTERN(__nx_thread_virtmem_unlock);
EXTERN(__nx_thread_virtmem_find_aslr);
EXTERN(__nx_thread_virtmem_find_stack);
EXTERN(__nx_thread_virtmem_find_code_memory);
EXTERN(__nx_thread_virtmem_add_reservation);
EXTERN(__nx_thread_virtmem_remove_reservation);

virtmemLock = __nx_thread_virtmem_lock;
virtmemUnlock = __nx_thread_virtmem_unlock;
virtmemFindAslr = __nx_thread_virtmem_find_aslr;
virtmemFindStack = __nx_thread_virtmem_find_stack;
virtmemFindCodeMemory = __nx_thread_virtmem_find_code_memory;
virtmemAddReservation = __nx_thread_virtmem_add_reservation;
virtmemRemoveReservation = __nx_thread_virtmem_remove_reservation;
