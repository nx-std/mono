#include <switch.h>

#include "../../harness.h"

/**
* @brief Sleeps the current thread for the given number of milliseconds.
* @param ms The number of milliseconds to sleep.
*/
static inline void threadSleepMs(int64_t ms) {
    svcSleepThread(ms * 1000000);
}

#define THREAD_COUNT 32
#define EXPECTED_BITFLAGS 0xFFFFFFFF

static Mutex g_mutex;
static CondVar g_condvar;
static bool g_wake_all = false;
static uint32_t g_bitflags = 0;

/**
 * Thread A function for Test #0003
 */
static void thread_func(void *arg) {
    const int64_t num = (int64_t) arg;

    mutexLock(&g_mutex);
    while (!g_wake_all) {
        condvarWait(&g_condvar, &g_mutex);
    }
    g_bitflags |= (1 << num);
    mutexUnlock(&g_mutex);
}

/**
 * A thread acquires a mutex and calls `wait_timeout()` with a short timeout. No thread should signal
 * the condition, and the test should confirm that the thread correctly resumes after the timeout
 * and re-acquires the mutex.
 */
test_rc_t test_0003_condvar_wait_wake_all(void) {
    Result rc = 0;

    //* Given
    // Initialize the test static mutex and condition variable
    mutexInit(&g_mutex);
    condvarInit(&g_condvar);

    // Create threads
    Thread threads[THREAD_COUNT];

    for (uint64_t i = 0; i < THREAD_COUNT; i++) {
        rc = threadCreate(&threads[i], thread_func, (void *) i, NULL, 0x10000, 0x2C, -2);
        if (R_FAILED(rc)) {
            goto test_cleanup;
        }
    }

    //* When
    // Start threads
    for (uint64_t i = 0; i < THREAD_COUNT; i++) {
        rc = threadStart(&threads[i]);
        if (R_FAILED(rc)) {
            goto test_cleanup;
        }
    }

    // Wait for all threads to lock the mutex
    threadSleepMs(50);

    // Mark the condition variable, and wake all threads
    mutexLock(&g_mutex);
    g_wake_all = true;
    condvarWakeAll(&g_condvar);
    mutexUnlock(&g_mutex);

    // Wait for all threads to set their bitflags
    threadSleepMs(50);

    //* Then
    // Assert all threads have set their bitflags
    if (g_bitflags != EXPECTED_BITFLAGS) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the mutex is unlocked
    if (g_mutex != INVALID_HANDLE) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the condition variable is initialized
    if (g_condvar != 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //* Cleanup
test_cleanup:
    for (uint64_t i = 0; i < THREAD_COUNT; i++) {
        threadWaitForExit(&threads[i]);
        threadClose(&threads[i]);
    }

    return rc;
} 
