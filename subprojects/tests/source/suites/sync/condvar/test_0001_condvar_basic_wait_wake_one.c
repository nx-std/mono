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

#define THREAD_A_TAG 0xA
#define THREAD_A_LOCK_DELAY_MS 300
#define THREAD_A_WAKE_ONE_DELAY_MS 100
#define THREAD_A_UNLOCK_DELAY_MS 100

#define THREAD_B_TAG 0xB
#define THREAD_B_LOCK_DELAY_MS 100
#define THREAD_B_WAIT_DELAY_MS 100

static Mutex g_mutex;
static CondVar g_condvar;
static int64_t g_shared_tag = -1;

/**
 * Thread A function for Test #0001
 */
static void thread_a_func(void *arg) {
    threadSleepMs(THREAD_A_LOCK_DELAY_MS);

    mutexLock(&g_mutex);
    g_shared_tag = THREAD_A_TAG;

    threadSleepMs(THREAD_A_WAKE_ONE_DELAY_MS);

    // Signal Thread B after setting the tag
    condvarWakeOne(&g_condvar);

    threadSleepMs(THREAD_A_UNLOCK_DELAY_MS);

    mutexUnlock(&g_mutex);
}

/**
 * Thread B function for Test #0001
 */
static void thread_b_func(void *arg) {
    threadSleepMs(THREAD_B_LOCK_DELAY_MS);
    mutexLock(&g_mutex);

    threadSleepMs(THREAD_B_WAIT_DELAY_MS);

    // Unlock the mutex and wait until Thread A signals, and the shared tag is set
    // to the expected value
    while (g_shared_tag != THREAD_A_TAG) {
        condvarWait(&g_condvar, &g_mutex);
    }

    g_shared_tag = THREAD_B_TAG;

    mutexUnlock(&g_mutex);
}

/**
 * A thread acquires a mutex, calls `wait()` on the condition variable, and another thread calls
 * `wake_one()` to resume the waiting thread. The test should confirm that only one thread is
 * successfully woken and resumes execution.
 */
test_rc_t test_0001_condvar_basic_wait_wake_one(void) {
    Result rc = 0;

    //* Given
    // Initialize the test static mutex and condition variable
    mutexInit(&g_mutex);
    condvarInit(&g_condvar);

    // Create threads
    Thread thread_a;
    Thread thread_b;

    rc = threadCreate(&thread_a, thread_a_func, NULL, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    rc = threadCreate(&thread_b, thread_b_func, NULL, NULL, 0x10000, 0x2C, -2);
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

    // Wait for Thread B to lock the mutex
    const int64_t t1 = t0 + THREAD_B_LOCK_DELAY_MS + 10; // t0 + 100ms (+ 10ms)
    threadSleepMs(t1 - t0);

    const uint32_t mutex_tag_t1 = g_mutex;
    const uint32_t condvar_tag_t1 = g_condvar;
    const int64_t shared_tag_t1 = g_shared_tag;

    // Wait for Thread B to wait on the condition variable
    const int64_t t2 = t1 + THREAD_B_WAIT_DELAY_MS + 10; // t1 + 100ms (+ 10ms)
    threadSleepMs(t2 - t1);

    const uint32_t mutex_tag_t2 = g_mutex;
    const uint32_t condvar_tag_t2 = g_condvar;
    const int64_t shared_tag_t2 = g_shared_tag;

    // Wait for Thread A to lock the mutex
    const int64_t t3 = t0 + THREAD_A_LOCK_DELAY_MS + 10; // t0 + 100ms (+ 10ms)
    threadSleepMs(t3 - t2);

    const uint32_t mutex_tag_t3 = g_mutex;
    const uint32_t condvar_tag_t3 = g_condvar;
    const int64_t shared_tag_t3 = g_shared_tag;

    // Wait for Thread A to wake Thread B
    const int64_t t4 = t3 + THREAD_A_WAKE_ONE_DELAY_MS + 10; // t3 + 100ms (+ 10ms)
    threadSleepMs(t4 - t3);

    const uint32_t mutex_tag_t4 = g_mutex;
    const uint32_t condvar_tag_t4 = g_condvar;
    const int64_t shared_tag_t4 = g_shared_tag;

    // Wait for Thread A to unlock the mutex, and Thread B to resume
    const int64_t t5 = t4 + THREAD_A_UNLOCK_DELAY_MS + 10; // t3 + 100ms (+ 10ms)
    threadSleepMs(t5 - t4);

    const uint32_t mutex_tag_t5 = g_mutex;
    const uint32_t condvar_tag_t5 = g_condvar;
    const int64_t shared_tag_t5 = g_shared_tag;

    //* Then
    // - T1
    // Assert that the mutex is locked by Thread B at *t1*, and there are no waiters
    if (!(mutex_tag_t1 != INVALID_HANDLE && (mutex_tag_t1 & HANDLE_WAIT_MASK) == 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the condition variable is initialized, but no threads are waiting
    if (condvar_tag_t1 != 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the shared tag is not set
    if (shared_tag_t1 != -1) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T2
    // Assert that the mutex was unlocked by the condition variable at *t2*
    if (mutex_tag_t2 != INVALID_HANDLE) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the condition variable is initialized, and one thread is waiting (Thread B)
    if (condvar_tag_t2 != 0x1) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the tag is not set
    if (shared_tag_t2 != -1) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T3
    // Assert that the mutex is locked by Thread A at *t3*, and there are no waiters
    if (!(mutex_tag_t3 != INVALID_HANDLE && (mutex_tag_t3 & HANDLE_WAIT_MASK) == 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the condition variable is initialized, and one thread is waiting (Thread B)
    if (condvar_tag_t3 != 0x1) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the shared tag was set by Thread A
    if (shared_tag_t3 != THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T4
    // Assert that the mutex is locked by Thread A at *t4*, and there are waiters (Thread B)
    if (!(mutex_tag_t4 != INVALID_HANDLE && (mutex_tag_t4 & HANDLE_WAIT_MASK) != 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert there are no waiters on the condition variable
    if (condvar_tag_t4 != 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the shared tag was set by Thread A
    if (shared_tag_t4 != THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T5
    // Assert that the mutex is unlocked at *t5*
    if (mutex_tag_t5 != INVALID_HANDLE) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert no threads are waiting on the condition variable
    if (condvar_tag_t5 != 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the shared tag was set by Thread B
    if (shared_tag_t5 != THREAD_B_TAG) {
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
