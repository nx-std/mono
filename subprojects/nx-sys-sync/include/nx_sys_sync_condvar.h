/**
 * @file condvar.h
 * @brief Condition variable synchronization primitive.
 * @author plutoo
 * @copyright libnx Authors
 */
#pragma once

#include <stdint.h>

#include <nx_sys_sync_mutex.h>


/// Result code.
typedef uint32_t Result;

/// Condition variable.
typedef uint32_t CondVar;

/**
 * @brief Initializes a condition variable.
 * @param[in] c Condition variable object.
 */
void __nx_sys_sync_condvar_init(CondVar* c);

/**
 * @brief Waits on a condition variable with a timeout.
 * @param[in] c Condition variable object.
 * @param[in] m Mutex object to use inside the condition variable.
 * @param[in] timeout Timeout in nanoseconds.
 * @return Result code (0xEA01 on timeout).
 * @remark On function return, the underlying mutex is acquired.
 */
Result __nx_sys_sync_condvar_wait_timeout(CondVar* c, Mutex* m, uint64_t timeout);

/**
 * @brief Waits on a condition variable.
 * @param[in] c Condition variable object.
 * @param[in] m Mutex object to use inside the condition variable.
 * @return Result code.
 * @remark On function return, the underlying mutex is acquired.
 */
Result __nx_sys_sync_condvar_wait(CondVar* c, Mutex* m);

/**
 * @brief Wakes up to the specified number of threads waiting on a condition variable.
 * @param[in] c Condition variable object.
 * @param[in] num Maximum number of threads to wake up (or -1 to wake them all up).
 * @return Result code.
 */
Result __nx_sys_sync_condvar_wake(CondVar* c, int num);

/**
 * @brief Wakes up a single thread waiting on a condition variable.
 * @param[in] c Condition variable object.
 * @return Result code.
 */
Result __nx_sys_sync_condvar_wake_one(CondVar* c);

/**
 * @brief Wakes up all thread waiting on a condition variable.
 * @param[in] c Condition variable object.
 * @return Result code.
 */
Result __nx_sys_sync_condvar_wake_all(CondVar* c);
