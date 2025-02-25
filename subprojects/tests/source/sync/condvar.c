#include <switch.h>

#include "../harness.h"

#define HANDLE_WAIT_MASK 0x40000000

/**
* @brief Sleeps the current thread for the given number of milliseconds.
* @param ms The number of milliseconds to sleep.
*/
static inline void threadSleepMs(int64_t ms) {
    svcSleepThread(ms * 1000000);
}

//<editor-fold desc="Test 0001: Condvar basic wait and notify one">

#define TEST_0001_THREAD_A_TAG 0xA
#define TEST_0001_THREAD_A_LOCK_DELAY_MS 300
#define TEST_0001_THREAD_A_WAKE_ONE_DELAY_MS 100
#define TEST_0001_THREAD_A_UNLOCK_DELAY_MS 100

#define TEST_0001_THREAD_B_TAG 0xB
#define TEST_0001_THREAD_B_LOCK_DELAY_MS 100
#define TEST_0001_THREAD_B_WAIT_DELAY_MS 100

static Mutex g_test_0001_mutex;
static CondVar g_test_0001_condvar;
static int64_t g_test_0001_shared_tag = -1;

/**
 * Thread A function for Test #0001
 */
void test_0001_condvar_thread_a_func(void *arg) {
    threadSleepMs(TEST_0001_THREAD_A_LOCK_DELAY_MS);

    mutexLock(&g_test_0001_mutex);
    g_test_0001_shared_tag = TEST_0001_THREAD_A_TAG;

    threadSleepMs(TEST_0001_THREAD_A_WAKE_ONE_DELAY_MS);

    // Signal Thread B after setting the tag
    condvarWakeOne(&g_test_0001_condvar);

    threadSleepMs(TEST_0001_THREAD_A_UNLOCK_DELAY_MS);

    mutexUnlock(&g_test_0001_mutex);
}

/**
 * Thread B function for Test #0001
 */
void test_0001_condvar_thread_b_func(void *arg) {
    threadSleepMs(TEST_0001_THREAD_B_LOCK_DELAY_MS);
    mutexLock(&g_test_0001_mutex);

    threadSleepMs(TEST_0001_THREAD_B_WAIT_DELAY_MS);

    // Unlock the mutex and wait until Thread A signals, and the shared tag is set
    // to the expected value
    while (g_test_0001_shared_tag != TEST_0001_THREAD_A_TAG) {
        condvarWait(&g_test_0001_condvar, &g_test_0001_mutex);
    }

    g_test_0001_shared_tag = TEST_0001_THREAD_B_TAG;

    mutexUnlock(&g_test_0001_mutex);
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
    mutexInit(&g_test_0001_mutex);
    condvarInit(&g_test_0001_condvar);

    // Create threads
    Thread thread_a;
    Thread thread_b;

    rc = threadCreate(&thread_a, test_0001_condvar_thread_a_func, NULL, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    rc = threadCreate(&thread_b, test_0001_condvar_thread_b_func, NULL, NULL, 0x10000, 0x2C, -2);
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
    const int64_t t1 = t0 + TEST_0001_THREAD_B_LOCK_DELAY_MS + 10; // t0 + 100ms (+ 10ms)
    threadSleepMs(t1 - t0);

    const uint32_t mutex_tag_t1 = g_test_0001_mutex;
    const uint32_t condvar_tag_t1 = g_test_0001_condvar;
    const int64_t shared_tag_t1 = g_test_0001_shared_tag;

    // Wait for Thread B to wait on the condition variable
    const int64_t t2 = t1 + TEST_0001_THREAD_B_WAIT_DELAY_MS + 10; // t1 + 100ms (+ 10ms)
    threadSleepMs(t2 - t1);

    const uint32_t mutex_tag_t2 = g_test_0001_mutex;
    const uint32_t condvar_tag_t2 = g_test_0001_condvar;
    const int64_t shared_tag_t2 = g_test_0001_shared_tag;

    // Wait for Thread A to lock the mutex
    const int64_t t3 = t0 + TEST_0001_THREAD_A_LOCK_DELAY_MS + 10; // t0 + 100ms (+ 10ms)
    threadSleepMs(t3 - t2);

    const uint32_t mutex_tag_t3 = g_test_0001_mutex;
    const uint32_t condvar_tag_t3 = g_test_0001_condvar;
    const int64_t shared_tag_t3 = g_test_0001_shared_tag;

    // Wait for Thread A to wake Thread B
    const int64_t t4 = t3 + TEST_0001_THREAD_A_WAKE_ONE_DELAY_MS + 10; // t3 + 100ms (+ 10ms)
    threadSleepMs(t4 - t3);

    const uint32_t mutex_tag_t4 = g_test_0001_mutex;
    const uint32_t condvar_tag_t4 = g_test_0001_condvar;
    const int64_t shared_tag_t4 = g_test_0001_shared_tag;

    // Wait for Thread A to unlock the mutex, and Thread B to resume
    const int64_t t5 = t4 + TEST_0001_THREAD_A_UNLOCK_DELAY_MS + 10; // t3 + 100ms (+ 10ms)
    threadSleepMs(t5 - t4);

    const uint32_t mutex_tag_t5 = g_test_0001_mutex;
    const uint32_t condvar_tag_t5 = g_test_0001_condvar;
    const int64_t shared_tag_t5 = g_test_0001_shared_tag;

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
    if (shared_tag_t3 != TEST_0001_THREAD_A_TAG) {
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
    if (shared_tag_t4 != TEST_0001_THREAD_A_TAG) {
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
    if (shared_tag_t5 != TEST_0001_THREAD_B_TAG) {
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

//<editor-fold desc="Test 0002: Condvar wait with timeout expiry">

#define TEST_0002_THREAD_A_LOCK_DELAY_MS 100
#define TEST_0002_THREAD_A_WAIT_DELAY_MS 100
#define TEST_0002_THREAD_A_WAIT_TIMEOUT_MS 200
#define TEST_0002_THREAD_A_UNLOCK_DELAY_MS 100

static Mutex g_test_0002_mutex;
static CondVar g_test_0002_condvar;

/**
 * Thread A function for Test #0002
 */
void test_0002_condvar_thread_a_func(void *arg) {
    threadSleepMs(TEST_0002_THREAD_A_LOCK_DELAY_MS);
    mutexLock(&g_test_0002_mutex);

    threadSleepMs(TEST_0002_THREAD_A_WAIT_DELAY_MS);
    condvarWaitTimeout(&g_test_0002_condvar, &g_test_0002_mutex, TEST_0002_THREAD_A_WAIT_TIMEOUT_MS * 1000000);

    threadSleepMs(TEST_0002_THREAD_A_UNLOCK_DELAY_MS);
    mutexUnlock(&g_test_0002_mutex);
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
    mutexInit(&g_test_0002_mutex);
    condvarInit(&g_test_0002_condvar);

    // Create threads
    Thread thread_a;

    rc = threadCreate(&thread_a, test_0002_condvar_thread_a_func, NULL, NULL, 0x10000, 0x2C, -2);
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
    const int64_t t1 = t0 + TEST_0002_THREAD_A_LOCK_DELAY_MS + 10; // t0 + 100ms (+ 10ms)
    threadSleepMs(t1 - t0);

    const uint32_t mutex_tag_t1 = g_test_0002_mutex;
    const uint32_t condvar_tag_t1 = g_test_0002_condvar;

    // Wait for Thread A to wait on the condition variable
    const int64_t t2 = t1 + TEST_0002_THREAD_A_WAIT_DELAY_MS + 10; // t1 + 100ms (+ 10ms)
    threadSleepMs(t2 - t1);

    const uint32_t mutex_tag_t2 = g_test_0002_mutex;
    const uint32_t condvar_tag_t2 = g_test_0002_condvar;

    // Wait 50% of the timeout period
    const int64_t t3 = t2 + TEST_0002_THREAD_A_WAIT_TIMEOUT_MS / 2 + 10; // t2 + 100ms (+ 10ms)
    threadSleepMs(t3 - t2);

    const uint32_t mutex_tag_t3 = g_test_0002_mutex;
    const uint32_t condvar_tag_t3 = g_test_0002_condvar;

    // Wait for the timeout to expire, and Thread A to resume
    // Mutex should be re-locked by Thread A
    const int64_t t4 = t2 + TEST_0002_THREAD_A_WAIT_TIMEOUT_MS + 10; // t2 + 200ms (+ 10ms)
    threadSleepMs(t4 - t3);

    const uint32_t mutex_tag_t4 = g_test_0002_mutex;
    const uint32_t condvar_tag_t4 = g_test_0002_condvar;

    // Wait for Thread A to unlock the mutex
    const int64_t t5 = t4 + TEST_0002_THREAD_A_UNLOCK_DELAY_MS + 10; // t4 + 100ms (+ 10ms)
    threadSleepMs(t5 - t4);

    const uint32_t mutex_tag_t5 = g_test_0002_mutex;
    const uint32_t condvar_tag_t5 = g_test_0002_condvar;

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

//</editor-fold>

//<editor-fold desc="Test 0003: Condvar wait and wake all">

#define TEST_0003_THREAD_COUNT 32
#define TEST_0003_EXPECTED_BITFLAGS 0xFFFFFFFF

static Mutex g_test_0003_mutex;
static CondVar g_test_0003_condvar;
static bool g_test_0003_wake_all = false;
static uint32_t g_test_0003_bitflags = 0;

/**
 * Thread A function for Test #0003
 */
void test_0003_condvar_thread_func(void *arg) {
    const int64_t num = (int64_t) arg;

    mutexLock(&g_test_0003_mutex);
    while (!g_test_0003_wake_all) {
        condvarWait(&g_test_0003_condvar, &g_test_0003_mutex);
    }
    g_test_0003_bitflags |= (1 << num);
    mutexUnlock(&g_test_0003_mutex);
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
    mutexInit(&g_test_0003_mutex);
    condvarInit(&g_test_0003_condvar);

    // Create threads
    Thread threads[TEST_0003_THREAD_COUNT];

    for (uint64_t i = 0; i < TEST_0003_THREAD_COUNT; i++) {
        rc = threadCreate(&threads[i], test_0003_condvar_thread_func, (void *) i, NULL, 0x10000, 0x2C, -2);
        if (R_FAILED(rc)) {
            goto test_cleanup;
        }
    }

    //* When
    // Start threads
    for (uint64_t i = 0; i < TEST_0003_THREAD_COUNT; i++) {
        rc = threadStart(&threads[i]);
        if (R_FAILED(rc)) {
            goto test_cleanup;
        }
    }

    // Wait for all threads to lock the mutex
    threadSleepMs(50);

    // Mark the condition variable, and wake all threads
    mutexLock(&g_test_0003_mutex);
    g_test_0003_wake_all = true;
    condvarWakeAll(&g_test_0003_condvar);
    mutexUnlock(&g_test_0003_mutex);

    // Wait for all threads to set their bitflags
    threadSleepMs(50);

    //* Then
    // Assert all threads have set their bitflags
    if (g_test_0003_bitflags != TEST_0003_EXPECTED_BITFLAGS) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the mutex is unlocked
    if (g_test_0003_mutex != INVALID_HANDLE) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert the condition variable is initialized
    if (g_test_0003_condvar != 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //* Cleanup
test_cleanup:
    for (uint64_t i = 0; i < TEST_0003_THREAD_COUNT; i++) {
        threadWaitForExit(&threads[i]);
        threadClose(&threads[i]);
    }

    return rc;
}

//</editor-fold>

//<editor-fold desc="Test 0004: Condvar sequential wait and signal">


#define TEST_0004_THREAD_COUNT 32
#define TEST_0004_THREAD_T2_DELAY_MS 200
#define TEST_0004_THREAD_T2_TOKEN_VALUE 15
#define TEST_0004_EXPECTED_BITFLAGS_T2 0x0000FFFF
#define TEST_0004_EXPECTED_BITFLAGS_T3 0xFFFFFFFF

static Mutex g_test_0004_mutex;
static CondVar g_test_0004_condvar;
static int64_t g_test_0004_token = -1;
static uint32_t g_test_0004_bitflags = 0;

/**
 * Thread function for Test #0004
 */
void test_0004_condvar_thread_func(void *arg) {
    const int64_t num = (int64_t) arg;

    // Lock the mutex
    mutexLock(&g_test_0004_mutex);

    // Wait for the right token
    while (g_test_0004_token != num) {
        condvarWait(&g_test_0004_condvar, &g_test_0004_mutex);
    }
    // Register that we have woken up
    g_test_0004_bitflags |= (1 << num);

    // On token #15, wait for 200ms
    if (g_test_0004_token == TEST_0004_THREAD_T2_TOKEN_VALUE) {
        threadSleepMs(TEST_0004_THREAD_T2_DELAY_MS);
    }

    // Increment the token, and wake the next thread
    if (num < TEST_0004_THREAD_COUNT - 1) {
        g_test_0004_token = num + 1;
        condvarWakeOne(&g_test_0004_condvar);
    }

    mutexUnlock(&g_test_0004_mutex);
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
    mutexInit(&g_test_0004_mutex);
    condvarInit(&g_test_0004_condvar);

    // Create threads
    Thread threads[TEST_0004_THREAD_COUNT];

    for (uint64_t i = 0; i < TEST_0004_THREAD_COUNT; i++) {
        rc = threadCreate(&threads[i], test_0004_condvar_thread_func, (void *) i, NULL, 0x10000, 0x2C, -2);
        if (R_FAILED(rc)) {
            goto test_cleanup;
        }
    }

    //* When
    // Start threads
    for (uint64_t i = 0; i < TEST_0004_THREAD_COUNT; i++) {
        rc = threadStart(&threads[i]);
        if (R_FAILED(rc)) {
            goto test_cleanup;
        }
    }

    const int64_t t0 = 0;

    // T1: Wait for all threads to lock the mutex, and wait for the condition variable
    const int64_t t1 = t0 + 50; // t0 + 50ms
    threadSleepMs(t1 - t0);

    const uint32_t mutex_tag_t1 = g_test_0004_mutex;
    const uint32_t condvar_tag_t1 = g_test_0004_condvar;
    const uint32_t bitflags_t1 = g_test_0004_bitflags;

    // Set the token to 0, and wake the first thread
    mutexLock(&g_test_0004_mutex);
    g_test_0004_token = 0;
    condvarWakeOne(&g_test_0004_condvar);
    mutexUnlock(&g_test_0004_mutex);

    // T2: Wait for 50% of the threads to set their bitflags
    const int64_t t2 = t1 + TEST_0004_THREAD_T2_DELAY_MS / 2 + 10; // t1 + 100ms (+ 10ms)
    threadSleepMs(t2 - t1);

    const uint32_t mutex_tag_t2 = g_test_0004_mutex;
    const uint32_t condvar_tag_t2 = g_test_0004_condvar;
    const uint32_t bitflags_t2 = g_test_0004_bitflags;

    // T3: Wait the rest of the threads to set their bitflags
    const int64_t t3 = t1 + TEST_0004_THREAD_T2_DELAY_MS + 10; // t1 + 200ms (+ 10ms)
    threadSleepMs(t3 - t2);

    const uint32_t mutex_tag_t3 = g_test_0004_mutex;
    const uint32_t condvar_tag_t3 = g_test_0004_condvar;
    const uint32_t bitflags_t3 = g_test_0004_bitflags;

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
    if (bitflags_t2 != TEST_0004_EXPECTED_BITFLAGS_T2) {
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
    if (bitflags_t3 != TEST_0004_EXPECTED_BITFLAGS_T3) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //* Cleanup
test_cleanup:
    for (uint64_t i = 0; i < TEST_0004_THREAD_COUNT; i++) {
        threadWaitForExit(&threads[i]);
        threadClose(&threads[i]);
    }

    return rc;
}

//</editor-fold>
