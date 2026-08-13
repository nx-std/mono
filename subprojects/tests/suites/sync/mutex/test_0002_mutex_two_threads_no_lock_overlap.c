#include <stdint.h>
#include <stdio.h>

#include <switch.h>

#include "../../harness.h"

#define HANDLE_WAIT_MASK 0x40000000

/**
* @brief Sleeps the current thread for the given number of milliseconds.
* @param ms The number of milliseconds to sleep.
*/
static inline void threadSleepMs(int64_t ms) {
    svcSleepThread(ms * 1000000);
}


//<editor-fold desc="Test 0002: Mutex two threads no lock overlap">

#define THREAD_A_TAG 1
#define THREAD_A_LOCK_DELAY_MS 100
#define THREAD_B_TAG 2
#define THREAD_B_LOCK_DELAY_MS 500

static Mutex g_mutex;
static int64_t g_shared_tag = -1;

typedef struct {
    int64_t tag; ///< The tag to set the shared variable to.
    int64_t lock_delay_ms; ///< The delay in milliseconds before locking the mutex and setting the shared variable.
} ThreadArgs;

/**
* Thread function for Test #0002
*
* Sets the shared variable to the tag after a delay.
*/
static void thread_func(void *arg) {
    const ThreadArgs *args = arg;

    threadSleepMs(args->lock_delay_ms);
    mutexLock(&g_mutex);

    g_shared_tag = args->tag;

    mutexUnlock(&g_mutex);
}

/**
* This test creates multiple threads that each set a shared variable to their thread number.
* The mutex locks DO NOT overlap, so the shared variable should be set to the thread number of the
* last thread to run.
*/
uint32_t test_0002_mutex_two_threads_no_lock_overlap(void) {
    Result rc = 0;

    //* Given
    // Initialize the test global mutex
    mutexInit(&g_mutex);

    // Create threads
    Thread thread_a;
    ThreadArgs thread_a_args = {
        .tag = THREAD_A_TAG,
        .lock_delay_ms = THREAD_A_LOCK_DELAY_MS
    };

    Thread thread_b;
    ThreadArgs thread_b_args = {
        .tag = THREAD_B_TAG,
        .lock_delay_ms = THREAD_B_LOCK_DELAY_MS
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

    // Wait for Thread A to lock the mutex, and set the shared tag, and unlock the mutex
    // t1 = t0 + 100ms (+ 10ms)
    threadSleepMs(THREAD_A_LOCK_DELAY_MS + 10);

    uint64_t shared_tag_t1 = g_shared_tag;

    // Wait for Thread B to lock the mutex, and set the shared tag, and unlock the mutex
    // t2 = t0 + 500ms (+ 10ms)
    threadSleepMs(THREAD_B_LOCK_DELAY_MS - THREAD_A_LOCK_DELAY_MS);

    uint64_t shared_tag_t2 = g_shared_tag;

    //* Then
    // Assert that the shared tag is set to THREAD_A_TAG at *t1*
    if (shared_tag_t1 != THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert that the shared tag is set to THREAD_B_TAG at *t2*
    if (shared_tag_t2 != THREAD_B_TAG) {
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

//</editor-fold>
