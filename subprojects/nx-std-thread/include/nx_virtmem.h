#pragma once

#include <stdint.h>
#include <stddef.h>

/// Address space reservation type (see \ref __nx_alloc_virtmem_add_reservation)
typedef struct VirtmemReservation VirtmemReservation;

/// Locks the virtual memory manager mutex.
void __nx_virtmem_lock(void);

/// Unlocks the virtual memory manager mutex.
void __nx_virtmem_unlock(void);

/**
 * @brief Finds a random slice of free general purpose address space.
 * @param size Desired size of the slice (rounded up to page alignment).
 * @param guard_size Desired size of the unmapped guard areas surrounding the slice (rounded up to page alignment).
 * @return Pointer to the slice of address space, or NULL on failure.
 * @note The virtual memory manager mutex must be held during the find-and-map process (see \ref __nx_alloc_virtmem_lock and \ref __nx_alloc_virtmem_unlock).
 */
void* __nx_virtmem_find_aslr(size_t size, size_t guard_size);

/**
 * @brief Finds a random slice of free stack address space.
 * @param size Desired size of the slice (rounded up to page alignment).
 * @param guard_size Desired size of the unmapped guard areas surrounding the slice (rounded up to page alignment).
 * @return Pointer to the slice of address space, or NULL on failure.
 * @note The virtual memory manager mutex must be held during the find-and-map process (see \ref __nx_alloc_virtmem_lock and \ref __nx_alloc_virtmem_unlock).
 */
void* __nx_virtmem_find_stack(size_t size, size_t guard_size);

/**
 * @brief Finds a random slice of free code memory address space.
 * @param size Desired size of the slice (rounded up to page alignment).
 * @param guard_size Desired size of the unmapped guard areas surrounding the slice (rounded up to page alignment).
 * @return Pointer to the slice of address space, or NULL on failure.
 * @note The virtual memory manager mutex must be held during the find-and-map process (see \ref __nx_alloc_virtmem_lock and \ref __nx_alloc_virtmem_unlock).
 */
void* __nx_virtmem_find_code_memory(size_t size, size_t guard_size);

/**
 * @brief Reserves a range of memory address space.
 * @param mem Pointer to the address space slice.
 * @param size Size of the slice.
 * @return Pointer to a reservation object, or NULL on failure.
 * @remark This function is intended to be used in lieu of a memory map operation when the memory won't be mapped straight away.
 * @note The virtual memory manager mutex must be held during the find-and-reserve process (see \ref __nx_alloc_virtmem_lock and \ref __nx_alloc_virtmem_unlock).
 */
VirtmemReservation* __nx_virtmem_add_reservation(void* mem, size_t size);

/**
 * @brief Releases a memory address space reservation.
 * @param rv Reservation to release.
 * @note The virtual memory manager mutex must be held before calling this function (see \ref __nx_alloc_virtmem_lock and \ref __nx_alloc_virtmem_unlock).
 */
void __nx_virtmem_remove_reservation(VirtmemReservation* rv); 
