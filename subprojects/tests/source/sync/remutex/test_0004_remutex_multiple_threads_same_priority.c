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

#define THREAD_C_TAG 0xC
#define THREAD_C_LOCK_DELAY_MS 300
#define THREAD_C_UNLOCK_DELAY_MS 100

static RMutex g_rmutex;
static int64_t g_shared_tag = -1;

typedef struct {
    int64_t tag;             ///< The tag to set the shared variable to.
    int64_t lock_delay_ms;   ///< The delay in milliseconds before locking the rmutex.
    int64_t unlock_delay_ms; ///< The delay in milliseconds before unlocking the rmutex.
} ThreadArgs;

/**
 * Thread function for Test #0004
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
 * This test creates multiple threads with the same priority that contend for
 * the same reentrant mutex, testing for race conditions and fairness.
 */
test_rc_t test_0004_remutex_multiple_threads_same_priority(void) {
    Result rc = 0;

    //* Given
    // Initialize the test global rmutex
    rmutexInit(&g_rmutex);

    // Create threads (all same priority: 0x2C)
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

    Thread thread_c;
    ThreadArgs thread_c_args = {
        .tag = THREAD_C_TAG,
        .lock_delay_ms = THREAD_C_LOCK_DELAY_MS,
        .unlock_delay_ms = THREAD_C_UNLOCK_DELAY_MS
    };

    rc = threadCreate(&thread_a, thread_func, &thread_a_args, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    rc = threadCreate(&thread_b, thread_func, &thread_b_args, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    rc = threadCreate(&thread_c, thread_func, &thread_c_args, NULL, 0x10000, 0x2C, -2);
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

    rc = threadStart(&thread_c);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    // T0: Time origin
    const int64_t t0 = 0;

    // T1: Wait for Thread A to lock the rmutex and set the shared tag
    const int64_t t1 = t0 + THREAD_A_LOCK_DELAY_MS + 10; // t0 + 100ms (+ 10ms)
    threadSleepMs(t1 - t0);

    const uint64_t shared_tag_t1 = g_shared_tag; // Should be THREAD_A_TAG

    // T2: Wait for Thread B to try to lock (blocked by A)
    const int64_t t2 = t0 + THREAD_B_LOCK_DELAY_MS + 10; // t0 + 200ms (+ 10ms)
    threadSleepMs(t2 - t1);

    const uint64_t shared_tag_t2 = g_shared_tag; // Should be THREAD_A_TAG

    // T3: Wait for Thread C to try to lock (blocked by A)
    const int64_t t3 = t0 + THREAD_C_LOCK_DELAY_MS + 10; // t0 + 300ms (+ 10ms)
    threadSleepMs(t3 - t2);

    const uint64_t shared_tag_t3 = g_shared_tag; // Should be THREAD_A_TAG

    // T4: Wait for Thread A to unlock, Thread B acquires
    const int64_t t4 = t1 + THREAD_A_UNLOCK_DELAY_MS + 10; // t1 + 500ms (+ 10ms)
    threadSleepMs(t4 - t3);

    const uint64_t shared_tag_t4 = g_shared_tag; // Should be THREAD_B_TAG

    // T5: Wait for Thread B to unlock, Thread C acquires
    const int64_t t5 = t4 + THREAD_B_UNLOCK_DELAY_MS + 10; // t4 + 100ms (+ 10ms)
    threadSleepMs(t5 - t4);

    const uint64_t shared_tag_t5 = g_shared_tag; // Should be THREAD_C_TAG

    // T6: Wait for Thread C to unlock
    const int64_t t6 = t5 + THREAD_C_UNLOCK_DELAY_MS + 10; // t5 + 100ms (+ 10ms)
    threadSleepMs(t6 - t5);

    const uint64_t shared_tag_t6 = g_shared_tag; // Should be THREAD_C_TAG

    //* Then
    // - T1: Thread A locks the rmutex
    if (shared_tag_t1 != THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T2: Thread B is blocked, A still has lock
    if (shared_tag_t2 != THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T3: Thread C is blocked, A still has lock
    if (shared_tag_t3 != THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T4: Thread A unlocked, Thread B acquired
    if (shared_tag_t4 != THREAD_B_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T5: Thread B unlocked, Thread C acquired
    if (shared_tag_t5 != THREAD_C_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T6: Thread C unlocked
    if (shared_tag_t6 != THREAD_C_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //* Clean-up
test_cleanup:
    threadWaitForExit(&thread_a);
    threadClose(&thread_a);
    threadWaitForExit(&thread_b);
    threadClose(&thread_b);
    threadWaitForExit(&thread_c);
    threadClose(&thread_c);

    return rc;
}
