/**
 * @file suite.h
 * @brief Test suite for reentrant mutexes (RMutex).
 *
 * This file declares the test cases for the reentrant mutex implementation.
 * The suite aims to cover various aspects of RMutex functionality.
 *
 * == Current Test Coverage ==
 * The existing tests cover:
 * - Basic single-threaded lock and unlock
 * - Multi-threaded scenarios without lock contention
 * - Multi-threaded scenarios with lock contention and blocking
 * - Behavior with multiple threads of the same priority, checking for race
 *   conditions and fairness
 * - Behavior with multiple threads of different priorities, relevant for
 *   priority scheduling and inversion avoidance
 * - The core reentrancy feature, where a single thread can acquire the lock
 *   multiple times
 *
 * == Coverage Enhancements ==
 * To ensure more comprehensive coverage, the following areas could be considered
 * for future test additions:
 *
 * `try_lock` semantics:
 * - Test for `remutex_try_lock()` successfully acquiring an unlocked mutex.
 * - Test for `remutex_try_lock()` successfully acquiring a mutex already
 *   locked by the *same* thread (reentrant try-lock).
 * - Test for `remutex_try_lock()` failing (returning an appropriate error or
 *   boolean false) when attempting to acquire a mutex locked by a *different*
 *   thread.
 *
 * Unlock behavior specifics:
 * - **Unlock Balancing for Reentrancy:** An explicit test to ensure the mutex
 *   is only fully released when the outermost lock is unlocked (i.e., the lock
 *   count for that thread returns to zero).
 * - **Attempting to Unlock a Non-Owned Mutex:** A test to verify the behavior
 *   (e.g., error code, no-op) when a thread tries to unlock a mutex it
 *   doesn't currently hold or one held by another thread.
 * - **Attempting to Unlock an Already Unlocked Mutex:** A test to verify the
 *   behavior when `remutex_unlock()` is called on a mutex that is not
 *   currently locked.
 */

#pragma once

#include "../../harness.h"

/**
 * Test reentrant mutex lock and unlock in a single thread.
 */
test_rc_t test_0001_remutex_lock_unlock_single_thread(void);

/**
 * This test creates two threads that access a shared resource protected by a
 * reentrant mutex. The locks do not overlap, ensuring basic multi-threaded
 * correctness.
 */
test_rc_t test_0002_remutex_two_threads_no_lock_overlap(void);

/**
 * This test creates two threads where one will block waiting for the other to
 * release the reentrant mutex, testing contention and blocking.
 */
test_rc_t test_0003_remutex_two_threads_with_lock_overlap(void);

/**
 * This test creates multiple threads with the same priority that contend for
 * the same reentrant mutex, testing for race conditions and fairness.
 */
test_rc_t test_0004_remutex_multiple_threads_same_priority(void);

/**
 * This test creates multiple threads with different priorities to test how the
 * reentrant mutex handles priority-based scheduling and avoids inversion.
 */
test_rc_t test_0005_remutex_multiple_threads_different_priority(void);

/**
 * Tests the core reentrancy feature, ensuring a single thread can lock the
 * mutex multiple times without deadlocking.
 */
test_rc_t test_0006_remutex_reentrancy_single_thread(void);

//
// Test suite for reentrant mutexes
//
static void sync_remutex_suite(void)
{
    TEST_SUITE("sync/remutex");

    TEST_CASE(
        "Test 0001: remutex_lock_unlock_single_thread",
        test_0001_remutex_lock_unlock_single_thread
    );
    TEST_CASE(
        "Test 0002: remutex_two_threads_no_lock_overlap",
        test_0002_remutex_two_threads_no_lock_overlap
    );
    TEST_CASE(
        "Test 0003: remutex_two_threads_with_lock_overlap",
        test_0003_remutex_two_threads_with_lock_overlap
    );
    TEST_CASE(
        "Test 0004: remutex_multiple_threads_same_priority",
        test_0004_remutex_multiple_threads_same_priority
    );
    TEST_CASE(
        "Test 0005: remutex_multiple_threads_different_priority",
        test_0005_remutex_multiple_threads_different_priority
    );
    TEST_CASE(
        "Test 0006: remutex_reentrancy_single_thread", 
        test_0006_remutex_reentrancy_single_thread
    );
} 
