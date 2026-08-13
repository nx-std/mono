#pragma once

#include "../../harness.h"

/**
 * Test semaphore wait and signal in a single thread.
 * 
 * This test covers:
 * - Basic Blocking Behavior: Tests a thread blocking on a semaphore with count 0
 * - Signaling Mechanism: Verifies that semaphoreSignal() properly unblocks a waiting thread
 * - Single Thread Control Flow: Ensures execution continues only after signaling
 * - Thread Synchronization: Demonstrates basic thread synchronization with semaphores
 */
test_rc_t test_0001_semaphore_wait_signal_single_thread(void);

/**
 * This test creates multiple threads that wait on a semaphore with an initial count.
 * Each thread decrements the semaphore count and performs its work.
 * 
 * This test covers:
 * - Initial Count Behavior: Tests semaphores initialized with a count > 0
 * - Concurrency Control: Ensures exactly N (initial count) threads can run concurrently
 * - Resource Management: Demonstrates controlling access to limited resources
 * - Multiple Thread Coordination: Tests behavior with multiple threads competing for resources
 * - Thread Cycling: Verifies waiting threads proceed as resources are released
 */
test_rc_t test_0002_semaphore_multiple_threads_initial_count(void);

/**
 * This test creates multiple producer and consumer threads.
 * Producer threads signal the semaphore, and consumer threads wait on it.
 * 
 * This test covers:
 * - Bounded Buffer: Uses semaphores to implement a thread-safe bounded buffer
 * - Multiple Semaphore Coordination: Uses two semaphores together (empty and full)
 * - Non-blocking Operations: Tests semaphoreTryWait() for non-blocking acquisition
 * - Producer-Consumer Pattern: Demonstrates the standard synchronization pattern
 * - Multiple Producers/Consumers: Tests with multiple threads on both sides
 * - Complete Cycle Verification: Ensures all produced items are properly consumed
 */
test_rc_t test_0003_semaphore_producer_consumer(void);

/**
 * Test suite for sync/semaphore.
 */
static void sync_semaphore_suite(void) {
    TEST_SUITE("sync/semaphore")

    TEST_CASE(
        "Test 0001: semaphore_wait_signal_single_thread",
        test_0001_semaphore_wait_signal_single_thread
    )
    TEST_CASE(
        "Test 0002: semaphore_multiple_threads_initial_count",
        test_0002_semaphore_multiple_threads_initial_count
    )
    TEST_CASE(
        "Test 0003: semaphore_producer_consumer",
        test_0003_semaphore_producer_consumer
    )
}
