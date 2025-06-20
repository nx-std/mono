/**
 * @file rwlock.h
 * @brief Read/write lock synchronization primitive.
 * @author plutoo, SciresM
 * @copyright libnx Authors
 */
#pragma once

#include "nx_sys_sync_mutex.h"
#include "nx_sys_sync_condvar.h"

/// Read/write lock structure.
typedef struct {
    Mutex mutex;
    CondVar condvar_reader_wait;
    CondVar condvar_writer_wait;
    uint32_t read_lock_count;
    uint32_t read_waiter_count;
    uint32_t write_lock_count;
    uint32_t write_waiter_count;
    uint32_t write_owner_tag;
} RwLock;

/**
 * @brief Initializes the read/write lock.
 * @param r Read/write lock object.
 */
void __nx_sys_sync_rwlock_init(RwLock* r);

/**
 * @brief Locks the read/write lock for reading.
 * @param r Read/write lock object.
 */
void __nx_sys_sync_rwlock_read_lock(RwLock* r);

/**
 * @brief Attempts to lock the read/write lock for reading without waiting.
 * @param r Read/write lock object.
 * @return 1 if the mutex has been acquired successfully, and 0 on contention.
 */
bool __nx_sys_sync_rwlock_try_read_lock(RwLock* r);

/**
 * @brief Unlocks the read/write lock for reading.
 * @param r Read/write lock object.
 */
void __nx_sys_sync_rwlock_read_unlock(RwLock* r);

/**
 * @brief Locks the read/write lock for writing.
 * @param r Read/write lock object.
 */
void __nx_sys_sync_rwlock_write_lock(RwLock* r);

/**
 * @brief Attempts to lock the read/write lock for writing without waiting.
 * @param r Read/write lock object.
 * @return 1 if the mutex has been acquired successfully, and 0 on contention.
 */
bool __nx_sys_sync_rwlock_try_write_lock(RwLock* r);

/**
 * @brief Unlocks the read/write lock for writing.
 * @param r Read/write lock object.
 */
void __nx_sys_sync_rwlock_write_unlock(RwLock* r);

/**
 * @brief Checks if the write lock is held by the current thread.
 * @param r Read/write lock object.
 * @return 1 if the current hold holds the write lock, and 0 if it does not.
 */
bool __nx_sys_sync_rwlock_is_write_lock_held_by_current_thread(RwLock* r);

/**
 * @brief Checks if the read/write lock is owned by the current thread.
 * @param r Read/write lock object.
 * @return 1 if the current hold holds the write lock or if it holds read locks acquired
 *         while it held the write lock, and 0 if it does not.
 */
bool __nx_sys_sync_rwlock_is_owned_by_current_thread(RwLock* r);
