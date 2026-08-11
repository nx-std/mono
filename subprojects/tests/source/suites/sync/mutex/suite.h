#pragma once

#include "../../harness.h"

/**
* Test mutex lock and unlock in a single thread.
*/
test_rc_t test_0001_mutex_lock_unlock_single_thread(void);

/**
* This test creates multiple threads that each set a shared variable to their thread number.
* The mutex locks DO NOT overlap, so the shared variable should be set to the thread number of the
* last thread to run.
*/
test_rc_t test_0002_mutex_two_threads_no_lock_overlap(void);

/**
* This test creates multiple threads that each set a shared variable to their thread number.
* The mutex locks DO overlap, so the shared variable should be set to the thread number of the
* last thread to lock the mutex.
*/
test_rc_t test_0003_mutex_two_threads_with_lock_overlap(void);

/**
* This test creates multiple threads that each set a shared variable to their thread number.
* The mutex locks DO overlap, so the shared variable should be set to the thread number of the
* last thread to lock the mutex. All threads have the same priority.
*/
test_rc_t test_0004_mutex_multiple_threads_same_priority(void);

/**
* This test creates multiple threads that each set a shared variable to their thread number.
* The mutex locks DO overlap, so the shared variable should be set to the thread number of the
* last thread to lock the mutex.
*
* Different priorities are used to test the priority inheritance mechanism.
*/
test_rc_t test_0005_mutex_multiple_threads_different_priority(void);

/**
 * Test suite for sync/mutex.
 */
static void sync_mutex_suite(void) {
    TEST_SUITE("sync/mutex");

    TEST_CASE(
        "Test 0001: mutex_lock_unlock_single_thread",
        test_0001_mutex_lock_unlock_single_thread
    )
    TEST_CASE(
        "Test 0002: mutex_two_threads_no_lock_overlap",
        test_0002_mutex_two_threads_no_lock_overlap
    )
    TEST_CASE(
        "Test 0003: mutex_two_threads_with_lock_overlap",
        test_0003_mutex_two_threads_with_lock_overlap
    )
    TEST_CASE(
        "Test 0004: mutex_multiple_threads_same_priority",
        test_0004_mutex_multiple_threads_same_priority
    )
    TEST_CASE(
        "Test 0005: mutex_multiple_threads_different_priority",
        test_0005_mutex_multiple_threads_different_priority
    )
}
