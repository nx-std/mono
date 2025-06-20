/**
 * @file barrier.h
 * @brief Multi-threading Barrier
 * @author tatehaga
 * @copyright libnx Authors
 */
#pragma once
#include <stdint.h>

#include "nx_sys_sync_mutex.h"
#include "nx_sys_sync_condvar.h"

/// Barrier structure.
typedef struct Barrier {
    uint64_t count;  ///< Number of threads to reach the barrier.
    uint64_t total;  ///< Number of threads to wait on.
    Mutex mutex;
    CondVar condvar;
} Barrier;

/**
 * @brief Initializes a barrier and the number of threads to wait on.
 * @param b Barrier object.
 * @param thread_count Initial value for the number of threads the barrier must wait for.
 */
void __nx_sys_sync_barrier_init(Barrier *b, uint64_t thread_count);

/**
 * @brief Forces threads to wait until all threads have called barrierWait.
 * @param b Barrier object.
 */
void __nx_sys_sync_barrier_wait(Barrier *b);
