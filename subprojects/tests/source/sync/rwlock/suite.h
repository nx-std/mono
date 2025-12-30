#pragma once

#include "../../harness.h"

/**
 * Test rwlock basic read lock functionality in a single thread.
 * 
 * This test covers:
 * - Basic Read Lock Behavior: Tests acquiring and releasing read locks
 * - Single Thread Control Flow: Ensures proper lock/unlock sequence
 * - Read Lock Mechanics: Verifies read locks work correctly without contention
 */
test_rc_t test_0001_rwlock_read_lock_single_thread(void);

/**
 * Test rwlock basic write lock functionality in a single thread.
 * 
 * This test covers:
 * - Basic Write Lock Behavior: Tests acquiring and releasing write locks
 * - Single Thread Control Flow: Ensures proper lock/unlock sequence
 * - Write Lock Mechanics: Verifies write locks work correctly without contention
 */
test_rc_t test_0002_rwlock_write_lock_single_thread(void);

/**
 * Test multiple readers can acquire read locks concurrently.
 * 
 * This test covers:
 * - Concurrent Read Access: Multiple threads can hold read locks simultaneously
 * - Shared Resource Access: Demonstrates non-exclusive read access patterns
 * - Read Lock Scalability: Tests behavior with multiple concurrent readers
 * - Thread Coordination: Ensures readers don't block each other
 */
test_rc_t test_0003_rwlock_multiple_readers_concurrent(void);

/**
 * Test write lock excludes all other access (readers and writers).
 * 
 * This test covers:
 * - Write Lock Exclusivity: Writer blocks all other readers and writers
 * - Mutual Exclusion: Only one writer can access the resource
 * - Reader-Writer Blocking: Readers must wait for writer to finish
 * - Resource Protection: Ensures exclusive access for modifications
 */
test_rc_t test_0004_rwlock_write_lock_exclusive(void);

/**
 * Test reader-writer priority scenarios and starvation prevention.
 * 
 * This test covers:
 * - Priority Handling: Tests how readers and writers are prioritized
 * - Starvation Prevention: Ensures no indefinite blocking of readers or writers
 * - Mixed Access Patterns: Combines read and write operations
 * - Fairness Mechanisms: Validates fair access scheduling
 */
test_rc_t test_0005_rwlock_reader_writer_priority(void);

/**
 * Test non-blocking try operations for both read and write locks.
 * 
 * This test covers:
 * - Try Lock Behavior: Tests rwlockTryReadLock and rwlockTryWriteLock
 * - Non-blocking Operations: Verifies try operations don't block when lock is held
 * - Contention Handling: Tests behavior when locks are unavailable
 * - Success Cases: Verifies try operations succeed when locks are available
 */
test_rc_t test_0006_rwlock_try_operations(void);

/**
 * Test read locks while holding write lock - unlock write first.
 * 
 * This test covers:
 * - Nested Lock Behavior: Thread holding write lock can acquire read locks
 * - Write-First Unlock Order: Releasing write lock before read locks
 * - Mixed Lock Types: Proper handling of both read and write locks by same thread
 * - Lock State Consistency: Ensures proper state transitions with mixed locks
 */
test_rc_t test_0007_rwlock_write_first_unlock(void);

/**
 * Test read locks while holding write lock - unlock reads first.
 * 
 * This test covers:
 * - Nested Lock Behavior: Thread holding write lock can acquire read locks
 * - Reads-First Unlock Order: Releasing read locks before write lock
 * - Mixed Lock Types: Proper handling of both read and write locks by same thread
 * - Lock State Consistency: Ensures proper state transitions with mixed locks
 */
test_rc_t test_0008_rwlock_reads_first_unlock(void);

/**
 * Test read locks while holding write lock - mixed unlock order.
 * 
 * This test covers:
 * - Nested Lock Behavior: Thread holding write lock can acquire read locks
 * - Mixed Unlock Order: Interleaved release of read and write locks
 * - Mixed Lock Types: Proper handling of both read and write locks by same thread
 * - Lock State Consistency: Ensures proper state transitions with complex unlock patterns
 */
test_rc_t test_0009_rwlock_mixed_unlock_order(void);

/**
 * Test RwLock ownership check functions.
 * 
 * This test covers:
 * - Write Lock Ownership: Tests rwlockIsWriteLockHeldByCurrentThread() functionality
 * - General Ownership: Tests rwlockIsOwnedByCurrentThread() functionality
 * - Thread Isolation: Verifies ownership functions work correctly across different threads
 * - Lock State Validation: Ensures ownership checks work with various lock combinations
 * - Cross-Thread Verification: Tests ownership from both current and other thread perspectives
 */
test_rc_t test_0010_rwlock_ownership_checks(void);

/**
 * Test suite for sync/rwlock.
 */
static void sync_rwlock_suite(void) {
    TEST_SUITE("sync/rwlock")

    TEST_CASE(
        "Test 0001: rwlock_read_lock_single_thread",
        test_0001_rwlock_read_lock_single_thread
    )
    TEST_CASE(
        "Test 0002: rwlock_write_lock_single_thread",
        test_0002_rwlock_write_lock_single_thread
    )
    TEST_CASE(
        "Test 0003: rwlock_multiple_readers_concurrent",
        test_0003_rwlock_multiple_readers_concurrent
    )
    TEST_CASE(
        "Test 0004: rwlock_write_lock_exclusive",
        test_0004_rwlock_write_lock_exclusive
    )
    TEST_CASE(
        "Test 0005: rwlock_reader_writer_priority",
        test_0005_rwlock_reader_writer_priority
    )
    TEST_CASE(
        "Test 0006: rwlock_try_operations",
        test_0006_rwlock_try_operations
    )
    TEST_CASE(
        "Test 0007: rwlock_write_first_unlock",
        test_0007_rwlock_write_first_unlock
    )
    TEST_CASE(
        "Test 0008: rwlock_reads_first_unlock",
        test_0008_rwlock_reads_first_unlock
    )
    TEST_CASE(
        "Test 0009: rwlock_mixed_unlock_order",
        test_0009_rwlock_mixed_unlock_order
    )
    TEST_CASE(
        "Test 0010: rwlock_ownership_checks",
        test_0010_rwlock_ownership_checks
    )
} 
