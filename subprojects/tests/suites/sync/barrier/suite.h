#pragma once

#include "../../harness.h"

/**
 * This test creates multiple threads that wait on a barrier.
 */
test_rc_t test_0001_barrier_sync_multiple_threads(void);


/**
 * Test suite for sync/barrier.
 */
static void sync_barrier_suite(void) {
    TEST_SUITE("sync/barrier")

    TEST_CASE(
        "Test 0001: barrier_sync_multiple_threads",
        test_0001_barrier_sync_multiple_threads
    )
}
