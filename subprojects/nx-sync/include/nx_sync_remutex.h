/**
 * @file remutex.h
 * @brief Reentrant Mutex synchronization primitive.
 * @author LNSD
 */
#pragma once

#include <stdbool.h>
#include <sys/lock.h>

/// Reentrant mutex datatype (defined in newlib).
typedef _LOCK_RECURSIVE_T ReentrantMutex;

/**
 * @brief Initializes a reentrant mutex.
 * @param m ReentrantMutex object.
 */
void __nx_sync_remutex_init(ReentrantMutex* m);

/**
 * @brief Locks a reentrant mutex.
 * @param m ReentrantMutex object.
 */
void __nx_sync_remutex_lock(ReentrantMutex* m);

/**
 * @brief Attempts to lock a reentrant mutex without waiting.
 * @param m ReentrantMutex object.
 * @return 1 if the mutex has been acquired successfully, and 0 on contention.
 */
bool __nx_sync_remutex_try_lock(ReentrantMutex* m);

/**
 * @brief Unlocks a reentrant mutex.
 * @param m ReentrantMutex object.
 */
void __nx_sync_remutex_unlock(ReentrantMutex* m); 
