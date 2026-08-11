#pragma once

#include "../../harness.h"

/**
 * A thread acquires a mutex, calls `wait()` on the condition variable, and another thread calls
 * `wake_one()` to resume the waiting thread. The test should confirm that only one thread is
 * successfully woken and resumes execution.
 */
test_rc_t test_0001_condvar_basic_wait_wake_one(void);

/**
 * A thread acquires a mutex and calls `wait_timeout()` with a short timeout. No thread should signal
 * the condition, and the test should confirm that the thread correctly resumes after the timeout and
 * re-acquires the mutex.
 */
test_rc_t test_0002_condvar_wait_timeout_expiry(void);

/**
 * A thread acquires a mutex and calls wait_timeout() with a short timeout. No thread should signal
 * the condition, and the test should confirm that the thread correctly resumes after the timeout
 * and re-acquires the mutex.
 */
test_rc_t test_0003_condvar_wait_wake_all(void);

/**
 * Multiple threads sequentially acquire the mutex, wait on the condition variable, and another
 * thread signals wake_one() multiple times. The test should verify that threads are woken in
 * the correct order, ensuring proper synchronization behavior.
 */
test_rc_t test_0004_condvar_sequential_wait_signal(void);


/**
 * Test suite for sync/condvar.
 */
static void sync_condvar_suite(void) {
    TEST_SUITE("sync/condvar");

    TEST_CASE(
        "Test 0001: condvar_basic_wait_wake_one",
        test_0001_condvar_basic_wait_wake_one
    )
    TEST_CASE(
        "Test 0002: condvar_wait_timeout_expiry",
        test_0002_condvar_wait_timeout_expiry
    )
    TEST_CASE(
        "Test 0003: condvar_wait_wake_all",
        test_0003_condvar_wait_wake_all
    )
    TEST_CASE(
        "Test 0004: condvar_sequential_wait_signal",
        test_0004_condvar_sequential_wait_signal
    )
}
