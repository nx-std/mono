#include <stdint.h>
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

#define THREAD_A_TAG 0xA
#define THREAD_A_LOCK_DELAY_MS 100
#define THREAD_A_UNLOCK_DELAY_MS 500

#define THREAD_B_TAG 0xB
#define THREAD_B_LOCK_DELAY_MS 200
#define THREAD_B_UNLOCK_DELAY_MS 100

static RMutex g_rmutex;
static int64_t g_shared_tag = -1;

typedef struct {
    int64_t tag;             ///< The tag to set the shared variable to.
    int64_t lock_delay_ms;   ///< The delay in milliseconds before locking the rmutex.
    int64_t unlock_delay_ms; ///< The delay in milliseconds before unlocking the rmutex.
} ThreadArgs;

/**
 * Thread function for Test #0003
 */
static void thread_func(void *arg) {
    const ThreadArgs *args = arg;

    threadSleepMs(args->lock_delay_ms);
    rmutexLock(&g_rmutex);

    g_shared_tag = args->tag;

    threadSleepMs(args->unlock_delay_ms);
    rmutexUnlock(&g_rmutex);
}

/**
 * This test creates multiple threads that each set a shared variable to their thread number.
 * The rmutex locks DO overlap, so Thread B should block until Thread A releases the lock.
 */
test_rc_t test_0003_remutex_two_threads_with_lock_overlap(void) {
    Result rc = 0;

    //* Given
    // Initialize the test global rmutex
    rmutexInit(&g_rmutex);

    // Create threads
    Thread thread_a;
    ThreadArgs thread_a_args = {
        .tag = THREAD_A_TAG,
        .lock_delay_ms = THREAD_A_LOCK_DELAY_MS,
        .unlock_delay_ms = THREAD_A_UNLOCK_DELAY_MS
    };

    Thread thread_b;
    ThreadArgs thread_b_args = {
        .tag = THREAD_B_TAG,
        .lock_delay_ms = THREAD_B_LOCK_DELAY_MS,
        .unlock_delay_ms = THREAD_B_UNLOCK_DELAY_MS
    };

    rc = threadCreate(&thread_a, thread_func, &thread_a_args, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    rc = threadCreate(&thread_b, thread_func, &thread_b_args, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    //* When
    // Start threads
    rc = threadStart(&thread_a);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    rc = threadStart(&thread_b);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    const int64_t t0 = 0;

    // Wait for Thread A to lock the rmutex and set the shared tag
    const int64_t t1 = t0 + THREAD_A_LOCK_DELAY_MS + 10; // t0 + 100ms (+ 10ms)
    threadSleepMs(t1 - t0);

    const uint64_t shared_tag_t1 = g_shared_tag; // Should be THREAD_A_TAG

    // Wait for Thread B to try to lock the rmutex (should be blocked by Thread A)
    const int64_t t2 = t0 + THREAD_B_LOCK_DELAY_MS + 10; // t0 + 200ms (+ 10ms)
    threadSleepMs(t2 - t1);

    const uint64_t shared_tag_t2 = g_shared_tag; // Should still be THREAD_A_TAG (B is blocked)

    // Wait for Thread A to unlock the rmutex, and Thread B to lock and set the shared tag
    const int64_t t3 = t1 + THREAD_A_UNLOCK_DELAY_MS + 10; // t1 + 500ms (+ 10ms)
    threadSleepMs(t3 - t2);

    const uint64_t shared_tag_t3 = g_shared_tag; // Should be THREAD_B_TAG

    // Wait for Thread B to unlock the rmutex
    const int64_t t4 = t3 + THREAD_B_UNLOCK_DELAY_MS + 10; // t3 + 100ms (+ 10ms)
    threadSleepMs(t4 - t3);

    const uint64_t shared_tag_t4 = g_shared_tag; // Should be THREAD_B_TAG

    //* Then
    // - T1: Thread A has the lock
    // Assert that the shared tag is set to THREAD_A_TAG at *t1*
    if (shared_tag_t1 != THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T2: Thread B is blocked, Thread A still has the lock
    // Assert that the shared tag is still THREAD_A_TAG at *t2*
    if (shared_tag_t2 != THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T3: Thread A unlocked, Thread B acquired and set the tag
    // Assert that the shared tag is set to THREAD_B_TAG at *t3*
    if (shared_tag_t3 != THREAD_B_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T4: Thread B unlocked
    // Assert that the shared tag is still THREAD_B_TAG at *t4*
    if (shared_tag_t4 != THREAD_B_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //* Clean-up
test_cleanup:
    threadWaitForExit(&thread_a);
    threadClose(&thread_a);
    threadWaitForExit(&thread_b);
    threadClose(&thread_b);

    return rc;
}
