#pragma once

#include "nx_tests_harness.h"

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
 * @brief Test that `tpidr_el0` is per-thread and survives a context switch.
 *
 * LLVM resolves a Rust `#[thread_local]` against `tpidr_el0`, which Horizon
 * does not maintain - `init_thread_vars` sets it. This checks it against the
 * recorded `ThreadVars.tls_ptr` on the main thread and on workers, that each
 * thread's is distinct, and that a value parked in a thread-local survives the
 * worker sleeping and being rescheduled.
 */
test_rc_t test_0003_thread_pointer_is_per_thread_and_preserved(void);

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
    TEST_CASE(
        "Test 0003: thread_pointer_is_per_thread_and_preserved",
        test_0003_thread_pointer_is_per_thread_and_preserved
    )
}
