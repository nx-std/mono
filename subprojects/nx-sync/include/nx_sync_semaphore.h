/**
 * @file semaphore.h
 * @brief Thread synchronization based on Mutex.
 * @author SciresM & Kevoot
 * @copyright libnx Authors
 */
#pragma once

#include <stdint.h>

#include "nx_sync_mutex.h"
#include "nx_sync_condvar.h"

/// Semaphore structure.
typedef struct Semaphore
{
    CondVar  condvar; ///< Condition variable object.
    Mutex    mutex;   ///< Mutex object.
    uint64_t count;   ///< Internal counter.
} Semaphore;

/**
 * @brief Initializes a __nx_sync_semaphore and its internal counter.
 * @param s Semaphore object.
 * @param initial_count initial value for internal counter (typically the # of free resources).
 */
void __nx_sync_semaphore_init(Semaphore *s, uint64_t initial_count);

/**
 * @brief Increments the Semaphore to allow other threads to continue
 * @param s Semaphore object.
 */
void __nx_sync_semaphore_signal(Semaphore *s);

/**
 * @brief Decrements Semaphore and waits if 0.
 * @param s Semaphore object.
 */
void __nx_sync_semaphore_wait(Semaphore *s);

/**
 * @brief Attempts to get lock without waiting.
 * @param s Semaphore object.
 * @return true if no wait and successful lock, false otherwise.
 */
bool __nx_sync_semaphore_try_wait(Semaphore *s);
