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

#define THREAD_COUNT 32
#define THREAD_T2_DELAY_MS 200
#define THREAD_T2_TOKEN_VALUE 15
#define EXPECTED_BITFLAGS_T2 0x0000FFFF
#define EXPECTED_BITFLAGS_T3 0xFFFFFFFF

static Mutex g_mutex;
static CondVar g_condvar;
static int64_t g_token = -1;
static uint32_t g_bitflags = 0;

/**
 * Thread function for Test #0004
 */
static void thread_func(void *arg) {
    const int64_t num = (int64_t) arg;

    // Lock the mutex
    mutexLock(&g_mutex);

    // Wait for the right token
    while (g_token != num) {
        condvarWait(&g_condvar, &g_mutex);
    }
    // Register that we have woken up
    g_bitflags |= (1 << num);

    // On token #15, wait for 200ms
    if (g_token == THREAD_T2_TOKEN_VALUE) {
        threadSleepMs(THREAD_T2_DELAY_MS);
    }

    // Increment the token, and wake the next thread
    if (num < THREAD_COUNT - 1) {
        g_token = num + 1;
        condvarWakeOne(&g_condvar);
    }

    mutexUnlock(&g_mutex);
}

/**
 * Multiple threads sequentially acquire the mutex, wait on the condition variable, and another
 * thread signals `wake_one()` multiple times. The test should verify that threads are woken in
 * the correct order, ensuring proper synchronization behavior.
 */
test_rc_t test_0004_condvar_sequential_wait_signal(void) {
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

    const int64_t t0 = 0;

    // T1: Wait for all threads to lock the mutex, and wait for the condition variable
    const int64_t t1 = t0 + 50; // t0 + 50ms
    threadSleepMs(t1 - t0);

    const uint32_t mutex_tag_t1 = g_mutex;
    const uint32_t condvar_tag_t1 = g_condvar;
    const uint32_t bitflags_t1 = g_bitflags;

    // Set the token to 0, and wake the first thread
    mutexLock(&g_mutex);
    g_token = 0;
    condvarWakeOne(&g_condvar);
    mutexUnlock(&g_mutex);

    // T2: Wait for 50% of the threads to set their bitflags
    const int64_t t2 = t1 + THREAD_T2_DELAY_MS / 2 + 10; // t1 + 100ms (+ 10ms)
    threadSleepMs(t2 - t1);

    const uint32_t mutex_tag_t2 = g_mutex;
    const uint32_t condvar_tag_t2 = g_condvar;
    const uint32_t bitflags_t2 = g_bitflags;

    // T3: Wait the rest of the threads to set their bitflags
    const int64_t t3 = t1 + THREAD_T2_DELAY_MS + 10; // t1 + 200ms (+ 10ms)
    threadSleepMs(t3 - t2);

    const uint32_t mutex_tag_t3 = g_mutex;
    const uint32_t condvar_tag_t3 = g_condvar;
    const uint32_t bitflags_t3 = g_bitflags;

    //* Then
    // - T1
    // Assert the mutex is unlocked
    if (mutex_tag_t1 != INVALID_HANDLE) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the condvar is initialized, and there are waiters
    if (condvar_tag_t1 == 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert all bitflags are unset
    if (bitflags_t1 != 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T2
    // Assert the mutex is locked, and has no waiters
    if (!(mutex_tag_t2 != INVALID_HANDLE && (mutex_tag_t2 & HANDLE_WAIT_MASK) == 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the condition variable has waiters
    if (condvar_tag_t2 == 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the first half of the threads have set their bitflags
    if (bitflags_t2 != EXPECTED_BITFLAGS_T2) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T3
    // Assert the mutex is unlocked
    if (mutex_tag_t3 != INVALID_HANDLE) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the condition variable has no waiters
    if (condvar_tag_t3 != 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert all threads have set their bitflags
    if (bitflags_t3 != EXPECTED_BITFLAGS_T3) {
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
