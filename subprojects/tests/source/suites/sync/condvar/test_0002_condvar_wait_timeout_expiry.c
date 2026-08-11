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

#define THREAD_A_LOCK_DELAY_MS 100
#define THREAD_A_WAIT_DELAY_MS 100
#define THREAD_A_WAIT_TIMEOUT_MS 200
#define THREAD_A_UNLOCK_DELAY_MS 100

static Mutex g_mutex;
static CondVar g_condvar;

/**
 * Thread A function for Test #0002
 */
static void thread_func(void *arg) {
    threadSleepMs(THREAD_A_LOCK_DELAY_MS);
    mutexLock(&g_mutex);

    threadSleepMs(THREAD_A_WAIT_DELAY_MS);
    condvarWaitTimeout(&g_condvar, &g_mutex, THREAD_A_WAIT_TIMEOUT_MS * 1000000);

    threadSleepMs(THREAD_A_UNLOCK_DELAY_MS);
    mutexUnlock(&g_mutex);
}

/**
 * A thread acquires a mutex and calls `wait_timeout()` with a short timeout. No thread should signal
 * the condition, and the test should confirm that the thread correctly resumes after the timeout and
 * re-acquires the mutex.
 */
test_rc_t test_0002_condvar_wait_timeout_expiry(void) {
    Result rc = 0;

    //* Given
    // Initialize the test static mutex and condition variable
    mutexInit(&g_mutex);
    condvarInit(&g_condvar);

    // Create threads
    Thread thread_a;

    rc = threadCreate(&thread_a, thread_func, NULL, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    //* When
    // Start threads
    rc = threadStart(&thread_a);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    const int64_t t0 = 0;

    // Wait for Thread A to lock the mutex
    const int64_t t1 = t0 + THREAD_A_LOCK_DELAY_MS + 10; // t0 + 100ms (+ 10ms)
    threadSleepMs(t1 - t0);

    const uint32_t mutex_tag_t1 = g_mutex;
    const uint32_t condvar_tag_t1 = g_condvar;

    // Wait for Thread A to wait on the condition variable
    const int64_t t2 = t1 + THREAD_A_WAIT_DELAY_MS + 10; // t1 + 100ms (+ 10ms)
    threadSleepMs(t2 - t1);

    const uint32_t mutex_tag_t2 = g_mutex;
    const uint32_t condvar_tag_t2 = g_condvar;

    // Wait 50% of the timeout period
    const int64_t t3 = t2 + THREAD_A_WAIT_TIMEOUT_MS / 2 + 10; // t2 + 100ms (+ 10ms)
    threadSleepMs(t3 - t2);

    const uint32_t mutex_tag_t3 = g_mutex;
    const uint32_t condvar_tag_t3 = g_condvar;

    // Wait for the timeout to expire, and Thread A to resume
    // Mutex should be re-locked by Thread A
    const int64_t t4 = t2 + THREAD_A_WAIT_TIMEOUT_MS + 10; // t2 + 200ms (+ 10ms)
    threadSleepMs(t4 - t3);

    const uint32_t mutex_tag_t4 = g_mutex;
    const uint32_t condvar_tag_t4 = g_condvar;

    // Wait for Thread A to unlock the mutex
    const int64_t t5 = t4 + THREAD_A_UNLOCK_DELAY_MS + 10; // t4 + 100ms (+ 10ms)
    threadSleepMs(t5 - t4);

    const uint32_t mutex_tag_t5 = g_mutex;
    const uint32_t condvar_tag_t5 = g_condvar;

    //* Then
    // - T1
    // Assert that the mutex is locked by Thread A at *t1*, and there are no waiters
    if (!(mutex_tag_t1 != INVALID_HANDLE && (mutex_tag_t1 & HANDLE_WAIT_MASK) == 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert condition variable is initialized, but no threads are waiting
    if (condvar_tag_t1 != 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T2
    // Assert that the mutex was unlocked by the condition variable at *t2*
    if (mutex_tag_t2 != INVALID_HANDLE) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the condition variable is initialized, and one thread is waiting (Thread A)
    if (condvar_tag_t2 != 0x1) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T3
    // Assert that the mutex is unlocked by the condition variable at *t3*
    if (mutex_tag_t3 != INVALID_HANDLE) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the condition variable is initialized, and one thread is waiting (Thread A)
    if (condvar_tag_t3 != 0x1) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T4
    // Assert the mutex is locked by Thread A at *t4*, no waiters
    if (!(mutex_tag_t4 != INVALID_HANDLE && (mutex_tag_t4 & HANDLE_WAIT_MASK) == 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the condition variable is initialized, and one thread is waiting (Thread A)
    if (condvar_tag_t4 != 1) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T5
    // Assert that the mutex is unlocked
    if (mutex_tag_t5 != INVALID_HANDLE) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the condition variable is initialized, one thread is waiting (Thread A)
    if (condvar_tag_t5 != 1) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //* Clean-up
test_cleanup:
    threadWaitForExit(&thread_a);
    threadClose(&thread_a);

    return rc;
} 
