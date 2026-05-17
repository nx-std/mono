#pragma once

#include "../harness.h"

/**
 * @brief Test that a thread detached while still running self-reclaims on exit.
 *
 * Creates many workers, detaching each while it is still asleep so the detach
 * wins the detach-vs-exit race; each detached worker then runs the Horizon
 * `__unmapself` self-reclaim port on exit. A broken stack switch or CAS faults
 * the process.
 */
test_rc_t test_0001_detach_running_thread_self_reclaims(void);

/**
 * @brief Test that detaching a thread that already exited reclaims it.
 *
 * Creates a worker, waits for it to exit, then detaches it — exercising the
 * detach-after-exit path where the detaching call reclaims the worker itself.
 */
test_rc_t test_0002_detach_after_exit_reclaims(void);

/**
 * Test suite for thread detachment and self-reclaim.
 */
static void thread_suite(void) {
    TEST_SUITE("thread");

    TEST_CASE(
        "Test 0001: detach_running_thread_self_reclaims",
        test_0001_detach_running_thread_self_reclaims
    )
    TEST_CASE(
        "Test 0002: detach_after_exit_reclaims",
        test_0002_detach_after_exit_reclaims
    )
}
