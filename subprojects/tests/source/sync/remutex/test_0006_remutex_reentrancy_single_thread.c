#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>

#include <switch.h>

#include "../../harness.h"

/**
 * @brief Sleeps the current thread for the given number of milliseconds.
 * @param ms The number of milliseconds to sleep.
 */
static inline void threadSleepMs(int64_t ms) {
    svcSleepThread(ms * 1000000);
}

#define REENTRY_DEPTH 3
#define OTHER_THREAD_TAG 0xBEEF
#define WAIT_DELAY_MS 100

static RMutex g_rmutex;
static int64_t g_shared_tag = -1;
static bool g_main_thread_released = false;

/**
 * Thread function: tries to acquire the rmutex after main thread releases it.
 */
static void other_thread_func(void *arg) {
    // Wait for main thread to signal that it fully released the lock
    while (!g_main_thread_released) {
        threadSleepMs(10);
    }

    // Now try to acquire - should succeed since main thread fully released
    rmutexLock(&g_rmutex);
    g_shared_tag = OTHER_THREAD_TAG;
    rmutexUnlock(&g_rmutex);
}

/**
 * Tests the core reentrancy feature, ensuring a single thread can lock the
 * mutex multiple times without deadlocking.
 *
 * - Main thread locks rmutex REENTRY_DEPTH times (should not deadlock)
 * - Main thread unlocks rmutex REENTRY_DEPTH times
 * - After full release, another thread should be able to acquire
 */
test_rc_t test_0006_remutex_reentrancy_single_thread(void) {
    Result rc = 0;

    //* Given
    // Initialize the test global rmutex
    rmutexInit(&g_rmutex);
    g_shared_tag = -1;
    g_main_thread_released = false;

    // Create another thread that will try to acquire after main releases
    Thread other_thread;
    rc = threadCreate(&other_thread, other_thread_func, NULL, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    //* When
    // Lock the rmutex multiple times (reentrant locking)
    for (int i = 0; i < REENTRY_DEPTH; i++) {
        rmutexLock(&g_rmutex);
        // After each lock, we should still be able to continue (no deadlock)
    }

    // Set a tag to prove we have the lock
    g_shared_tag = 0xAAAA;

    // Check that counter reflects reentrant depth
    // RMutex counter should be REENTRY_DEPTH after REENTRY_DEPTH locks
    if (g_rmutex.counter != REENTRY_DEPTH) {
        rc = TEST_ASSERTION_FAILED;
        goto test_unlock;
    }

    // Start the other thread (it will wait for us to release)
    rc = threadStart(&other_thread);
    if (R_FAILED(rc)) {
        goto test_unlock;
    }

    // Unlock REENTRY_DEPTH - 1 times (lock should still be held)
    for (int i = 0; i < REENTRY_DEPTH - 1; i++) {
        rmutexUnlock(&g_rmutex);
    }

    // Counter should be 1 now (one lock remaining)
    if (g_rmutex.counter != 1) {
        rc = TEST_ASSERTION_FAILED;
        goto test_unlock;
    }

    // Give other thread a chance to try to acquire (it should be blocked)
    threadSleepMs(WAIT_DELAY_MS);

    // Shared tag should still be our value (other thread couldn't acquire)
    if (g_shared_tag != 0xAAAA) {
        rc = TEST_ASSERTION_FAILED;
        goto test_unlock;
    }

    // Final unlock - fully releases the lock
    rmutexUnlock(&g_rmutex);

    // Counter should be 0 now
    if (g_rmutex.counter != 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Signal to other thread that we released
    g_main_thread_released = true;

    // Wait for other thread to acquire and set its tag
    threadSleepMs(WAIT_DELAY_MS);

    //* Then
    // Other thread should have acquired and set the tag
    if (g_shared_tag != OTHER_THREAD_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    goto test_cleanup;

test_unlock:
    // Emergency unlock in case of early failure
    while (g_rmutex.counter > 0) {
        rmutexUnlock(&g_rmutex);
    }
    g_main_thread_released = true;

test_cleanup:
    threadWaitForExit(&other_thread);
    threadClose(&other_thread);

    return rc;
}
