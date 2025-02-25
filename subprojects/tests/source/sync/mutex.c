#include <stdint.h>
#include <stdio.h>

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

//<editor-fold desc="Test 0001: Mutex lock unlock single thread">

#define TEST_0001_THREAD_TAG 42

static Mutex g_test_0001_mutex;
static int64_t g_test_0001_shared_tag = -1;

/**
 * Thread function for Test #0001
 */
void test_0001_thread_func(void *arg) {
    const int64_t num = (int64_t) arg;

    mutexLock(&g_test_0001_mutex);
    g_test_0001_shared_tag = num;
    mutexUnlock(&g_test_0001_mutex);
}


/**
* Test mutex lock and unlock in a single thread.
*/
uint32_t test_0001_mutex_lock_unlock_single_thread(void) {
    Result rc = 0;

    //* Given
    // Initialize the test global mutex
    mutexInit(&g_test_0001_mutex);

    // Create a thread
    Thread thread;
    rc = threadCreate(&thread, test_0001_thread_func, (void *) TEST_0001_THREAD_TAG, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    //* When
    // Start the thread
    rc = threadStart(&thread);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    // Wait for the thread to set the shared tag (10ms should be enough)
    threadSleepMs(10);

    uint64_t shared_tag = g_test_0001_shared_tag;

    //* Then
    // Assert that the shared tag is set to TEST_0001_THREAD_TAG
    if (shared_tag != TEST_0001_THREAD_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

test_cleanup:
    threadWaitForExit(&thread);
    threadClose(&thread);

    return rc;
}

//</editor-fold>

//<editor-fold desc="Test 0002: Mutex two threads no lock overlap">

#define TEST_0002_THREAD_A_TAG 1
#define TEST_0002_THREAD_A_LOCK_DELAY_MS 100
#define TEST_0002_THREAD_B_TAG 2
#define TEST_0002_THREAD_B_LOCK_DELAY_MS 500

static Mutex g_test_0002_mutex;
static int64_t g_test_0002_shared_tag = -1;

typedef struct {
    int64_t tag; ///< The tag to set the shared variable to.
    int64_t lock_delay_ms; ///< The delay in milliseconds before locking the mutex and setting the shared variable.
} Test0002_ThreadArgs;

/**
* Thread function for Test #0002
*
* Sets the shared variable to the tag after a delay.
*/
void test_0002_thread_func(void *arg) {
    const Test0002_ThreadArgs *args = arg;

    threadSleepMs(args->lock_delay_ms);
    mutexLock(&g_test_0002_mutex);

    g_test_0002_shared_tag = args->tag;

    mutexUnlock(&g_test_0002_mutex);
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
    mutexInit(&g_test_0002_mutex);

    // Create threads
    Thread thread_a;
    Test0002_ThreadArgs thread_a_args = {
        .tag = TEST_0002_THREAD_A_TAG,
        .lock_delay_ms = TEST_0002_THREAD_A_LOCK_DELAY_MS
    };

    Thread thread_b;
    Test0002_ThreadArgs thread_b_args = {
        .tag = TEST_0002_THREAD_B_TAG,
        .lock_delay_ms = TEST_0002_THREAD_B_LOCK_DELAY_MS
    };

    rc = threadCreate(&thread_a, test_0002_thread_func, &thread_a_args, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    rc = threadCreate(&thread_b, test_0002_thread_func, &thread_b_args, NULL, 0x10000, 0x2C, -2);
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
    threadSleepMs(TEST_0002_THREAD_A_LOCK_DELAY_MS + 10);

    uint64_t shared_tag_t1 = g_test_0002_shared_tag;

    // Wait for Thread B to lock the mutex, and set the shared tag, and unlock the mutex
    // t2 = t0 + 500ms (+ 10ms)
    threadSleepMs(TEST_0002_THREAD_B_LOCK_DELAY_MS - TEST_0002_THREAD_A_LOCK_DELAY_MS);

    uint64_t shared_tag_t2 = g_test_0002_shared_tag;

    //* Then
    // Assert that the shared tag is set to TEST_0002_THREAD_A_TAG at *t1*
    if (shared_tag_t1 != TEST_0002_THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert that the shared tag is set to TEST_0002_THREAD_B_TAG at *t2*
    if (shared_tag_t2 != TEST_0002_THREAD_B_TAG) {
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

//<editor-fold desc="Test 0003: Mutex two threads with lock overlap">

#define TEST_0003_THREAD_A_TAG 0xA
#define TEST_0003_THREAD_A_LOCK_DELAY_MS 100
#define TEST_0003_THREAD_A_UNLOCK_DELAY_MS 500

#define TEST_0003_THREAD_B_TAG 0xB
#define TEST_0003_THREAD_B_LOCK_DELAY_MS 200
#define TEST_0003_THREAD_B_UNLOCK_DELAY_MS 100

static Mutex g_test_0003_mutex;
static int64_t g_test_0003_shared_tag = -1;

typedef struct {
    int64_t tag; ///< The tag to set the shared variable to.
    int64_t lock_delay_ms; ///< The delay in milliseconds before locking the mutex and setting the shared variable.
    int64_t unlock_delay_ms; ///< The delay in milliseconds before unlocking the mutex.
} Test0003_ThreadArgs;

/**
* Thread function for Test #0003
*/
void test_0003_thread_func(void *arg) {
    const Test0003_ThreadArgs *args = arg;

    threadSleepMs(args->lock_delay_ms);
    mutexLock(&g_test_0003_mutex);

    g_test_0003_shared_tag = args->tag;

    threadSleepMs(args->unlock_delay_ms);
    mutexUnlock(&g_test_0003_mutex);
}

/**
* This test creates multiple threads that each set a shared variable to their thread number.
* The mutex locks DO overlap, so the shared variable should be set to the thread number of the
* last thread to lock the mutex.
*/
uint32_t test_0003_mutex_two_threads_with_lock_overlap(void) {
    Result rc = 0;

    //* Given
    // Initialize the test global mutex
    mutexInit(&g_test_0003_mutex);

    // Create threads
    Thread thread_a;
    Test0003_ThreadArgs thread_a_args = {
        .tag = TEST_0003_THREAD_A_TAG,
        .lock_delay_ms = TEST_0003_THREAD_A_LOCK_DELAY_MS,
        .unlock_delay_ms = TEST_0003_THREAD_A_UNLOCK_DELAY_MS
    };

    Thread thread_b;
    Test0003_ThreadArgs thread_b_args = {
        .tag = TEST_0003_THREAD_B_TAG,
        .lock_delay_ms = TEST_0003_THREAD_B_LOCK_DELAY_MS,
        .unlock_delay_ms = TEST_0003_THREAD_B_UNLOCK_DELAY_MS
    };

    rc = threadCreate(&thread_a, test_0003_thread_func, &thread_a_args, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    rc = threadCreate(&thread_b, test_0003_thread_func, &thread_b_args, NULL, 0x10000, 0x2C, -2);
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

    // Wait for Thread A to lock the mutex, and set the shared tag
    const int64_t t1 = t0 + TEST_0003_THREAD_A_LOCK_DELAY_MS + 10; // t0 + 100ms (+ 10ms)
    threadSleepMs(t1 - t0);

    const uint32_t mutex_tag_t1 = g_test_0003_mutex;
    const uint64_t shared_tag_t1 = g_test_0003_shared_tag; // Should be TEST_0003_THREAD_A_TAG

    // Wait for Thread B to try to lock the mutex, mutex should be locked by Thread A and marked as contended
    const int64_t t2 = t0 + TEST_0003_THREAD_B_LOCK_DELAY_MS + 10; // t0 + 200ms (+ 10ms)
    threadSleepMs(t2 - t1);

    const uint32_t mutex_tag_t2 = g_test_0003_mutex;
    const uint64_t shared_tag_t2 = g_test_0003_shared_tag; // Should be TEST_0003_THREAD_A_TAG

    // Wait for Thread A to unlock the mutex, and Thread B to lock the mutex and set the shared tag
    const int64_t t3 = t1 + TEST_0003_THREAD_A_UNLOCK_DELAY_MS + 10; // t1 + 500ms (+ 10ms)
    threadSleepMs(t3 - t2);

    const uint32_t mutex_tag_t3 = g_test_0003_mutex;
    const uint64_t shared_tag_t3 = g_test_0003_shared_tag; // Should be TEST_0003_THREAD_B_TAG

    // Wait for Thread B to unlock the mutex
    const int64_t t4 = t3 + TEST_0003_THREAD_B_UNLOCK_DELAY_MS + 10; // t3 + 100ms (+ 10ms)
    threadSleepMs(t4 - t3);

    const uint32_t mutex_tag_t4 = g_test_0003_mutex;
    const uint64_t shared_tag_t4 = g_test_0003_shared_tag; // Should be TEST_0003_THREAD_B_TAG

    //* Then
    // - T1
    // Assert that the mutex is locked by Thread A at *t1*, and there are no waiters
    if (!(mutex_tag_t1 != INVALID_HANDLE && (mutex_tag_t1 & HANDLE_WAIT_MASK) == 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert that the shared tag is set to TEST_0003_THREAD_A_TAG at *t1*
    if (shared_tag_t1 != TEST_0003_THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T2
    // Assert that the mutex is locked by Thread A at *t2*, and there are waiters
    if (!(mutex_tag_t2 != INVALID_HANDLE && (mutex_tag_t2 & HANDLE_WAIT_MASK) != 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert that the shared tag is set to TEST_0003_THREAD_A_TAG at *t2*
    if (shared_tag_t2 != TEST_0003_THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T3
    // Assert that the mutex is locked by Thread B at *t3*, and there are no waiters
    if (!(mutex_tag_t3 != INVALID_HANDLE && (mutex_tag_t3 & HANDLE_WAIT_MASK) == 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert that the shared tag is set to TEST_0003_THREAD_B_TAG at *t3*
    if (shared_tag_t3 != TEST_0003_THREAD_B_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T4
    // Assert that the mutex is unlocked at *t4*
    if (mutex_tag_t4 != INVALID_HANDLE) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert that the shared tag is set to TEST_0003_THREAD_B_TAG at *t4*
    if (shared_tag_t4 != TEST_0003_THREAD_B_TAG) {
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

//<editor-fold>

//<Editor fold desc="test 0004: Mutex multiple threads same priority">

#define TEST_0004_THREAD_A_TAG 0xA
#define TEST_0004_THREAD_A_LOCK_DELAY_MS 100
#define TEST_0004_THREAD_A_UNLOCK_DELAY_MS 500

#define TEST_0004_THREAD_B_TAG 0xB
#define TEST_0004_THREAD_B_LOCK_DELAY_MS 200
#define TEST_0004_THREAD_B_UNLOCK_DELAY_MS 100

#define TEST_0004_THREAD_C_TAG 0xC
#define TEST_0004_THREAD_C_LOCK_DELAY_MS 300
#define TEST_0004_THREAD_C_UNLOCK_DELAY_MS 100


static Mutex g_test_0004_mutex;
static int64_t g_test_0004_shared_tag = -1;

typedef struct {
    int64_t tag; ///< The tag to set the shared variable to.
    int64_t lock_delay_ms; ///< The delay in milliseconds before locking the mutex and setting the shared variable.
    int64_t unlock_delay_ms; ///< The delay in milliseconds before unlocking the mutex.
} Test0004_ThreadArgs;

/**
* Thread function for Test #0004
*/
void test_0004_thread_func(void *arg) {
    const Test0004_ThreadArgs *args = arg;

    threadSleepMs(args->lock_delay_ms);
    mutexLock(&g_test_0004_mutex);

    g_test_0004_shared_tag = args->tag;

    threadSleepMs(args->unlock_delay_ms);
    mutexUnlock(&g_test_0004_mutex);
}

/**
* This test creates multiple threads that each set a shared variable to their thread number.
* The mutex locks DO overlap, so the shared variable should be set to the thread number of the
* last thread to lock the mutex.
*/
uint32_t test_0004_mutex_multiple_threads_same_priority(void) {
    Result rc = 0;

    //* Given
    // Initialize the test global mutex
    mutexInit(&g_test_0004_mutex);

    // Create threads
    Thread thread_a;
    Test0004_ThreadArgs thread_a_args = {
        .tag = TEST_0004_THREAD_A_TAG,
        .lock_delay_ms = TEST_0004_THREAD_A_LOCK_DELAY_MS,
        .unlock_delay_ms = TEST_0004_THREAD_A_UNLOCK_DELAY_MS
    };

    Thread thread_b;
    Test0004_ThreadArgs thread_b_args = {
        .tag = TEST_0004_THREAD_B_TAG,
        .lock_delay_ms = TEST_0004_THREAD_B_LOCK_DELAY_MS,
        .unlock_delay_ms = TEST_0004_THREAD_B_UNLOCK_DELAY_MS
    };

    Thread thread_c;
    Test0003_ThreadArgs thread_c_args = {
        .tag = TEST_0004_THREAD_C_TAG,
        .lock_delay_ms = TEST_0004_THREAD_C_LOCK_DELAY_MS,
        .unlock_delay_ms = TEST_0004_THREAD_C_UNLOCK_DELAY_MS
    };

    rc = threadCreate(&thread_a, test_0004_thread_func, &thread_a_args, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    rc = threadCreate(&thread_b, test_0004_thread_func, &thread_b_args, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    rc = threadCreate(&thread_c, test_0004_thread_func, &thread_c_args, NULL, 0x10000, 0x2C, -2);
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

    // T1: Wait for Thread A to lock the mutex, and set the shared tag
    const int64_t t1 = t0 + TEST_0004_THREAD_A_LOCK_DELAY_MS + 10; // t0 + 100ms (+ 10ms)
    threadSleepMs(t1 - t0);

    const uint32_t mutex_tag_t1 = g_test_0004_mutex;
    const uint64_t shared_tag_t1 = g_test_0004_shared_tag; // Should be TEST_0004_THREAD_A_TAG

    // T2: Wait for Thread B to try to lock the mutex, mutex should be locked by Thread A and marked as contended
    const int64_t t2 = t0 + TEST_0004_THREAD_B_LOCK_DELAY_MS + 10; // t0 + 200ms (+ 10ms)
    threadSleepMs(t2 - t1);

    const uint32_t mutex_tag_t2 = g_test_0004_mutex;
    const uint64_t shared_tag_t2 = g_test_0004_shared_tag; // Should be TEST_0004_THREAD_A_TAG

    // T3: Wait for Thread C to try to lock the mutex, mutex should be locked by Thread A and marked as contended
    const int64_t t3 = t0 + TEST_0004_THREAD_C_LOCK_DELAY_MS + 10; // t0 + 300ms (+ 10ms)
    threadSleepMs(t3 - t2);

    const uint32_t mutex_tag_t3 = g_test_0004_mutex;
    const uint64_t shared_tag_t3 = g_test_0004_shared_tag; // Should be TEST_0004_THREAD_A_TAG

    // T4: Wait for Thread A to unlock the mutex, and Thread B to lock the mutex and set the shared tag
    const int64_t t4 = t1 + TEST_0004_THREAD_A_UNLOCK_DELAY_MS + 10; // t1 + 500ms (+ 10ms)
    threadSleepMs(t4 - t3);

    const uint32_t mutex_tag_t4 = g_test_0004_mutex;
    const uint64_t shared_tag_t4 = g_test_0004_shared_tag; // Should be TEST_0004_THREAD_B_TAG

    // T5: Wait for Thread B to unlock the mutex, and Thread C to lock the mutex and set the shared tag
    const int64_t t5 = t4 + TEST_0004_THREAD_B_UNLOCK_DELAY_MS + 10; // t4 + 100ms (+ 10ms)
    threadSleepMs(t5 - t4);

    const uint32_t mutex_tag_t5 = g_test_0004_mutex;
    const uint64_t shared_tag_t5 = g_test_0004_shared_tag; // Should be TEST_0004_THREAD_B_TAG

    // T6: Wait for Thread C to unlock the mutex
    const int64_t t6 = t5 + TEST_0004_THREAD_C_UNLOCK_DELAY_MS + 10; // t5 + 100ms (+ 10ms)
    threadSleepMs(t6 - t5);

    const uint32_t mutex_tag_t6 = g_test_0004_mutex;
    const uint64_t shared_tag_t6 = g_test_0004_shared_tag; // Should be TEST_0004_THREAD_C_TAG

    //* Then
    //- T1: Thread A locks the mutex
    // Assert that the mutex is locked (by Thread A), and there are no waiters
    if (!(mutex_tag_t1 != INVALID_HANDLE && (mutex_tag_t1 & HANDLE_WAIT_MASK) == 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert that the shared tag is set by Thread A
    if (shared_tag_t1 != TEST_0004_THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //- T2: Thread B tries to lock the mutex
    // Assert that the mutex is locked (by Thread A), and there are waiters (Thread B)
    if (!(mutex_tag_t2 != INVALID_HANDLE && (mutex_tag_t2 & HANDLE_WAIT_MASK) != 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert that the shared tag was set by Thread A
    if (shared_tag_t2 != TEST_0004_THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //- T3: Thread C tries to lock the mutex
    // Assert that the mutex is locked (by Thread B), and there are waiters (Threads B and C)
    if (!(mutex_tag_t3 != INVALID_HANDLE && (mutex_tag_t3 & HANDLE_WAIT_MASK) != 0)) {
        rc = TEST_ASSERTION_FAILED - 4;
        goto test_cleanup;
    }

    // Assert that the shared tag was set by Thread A
    if (shared_tag_t3 != TEST_0004_THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //- T4: Thread A unlocks the mutex, Thread B locks the mutex
    // Assert that the mutex is locked (by Thread B), and there are waiters (Thread C)
    if (!(mutex_tag_t4 != INVALID_HANDLE && (mutex_tag_t4 & HANDLE_WAIT_MASK) != 0)) {
        rc = TEST_ASSERTION_FAILED - 6;
        goto test_cleanup;
    }

    // Assert that the shared tag was set by Thread B
    if (shared_tag_t4 != TEST_0004_THREAD_B_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //- T5
    // Assert that the mutex is locked (by Thread C), and there are no waiters
    if (!(mutex_tag_t5 != INVALID_HANDLE && (mutex_tag_t5 & HANDLE_WAIT_MASK) == 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert that the shared tag was set by Thread C
    if (shared_tag_t5 != TEST_0004_THREAD_C_TAG) {
        rc = TEST_ASSERTION_FAILED - 9;
        goto test_cleanup;
    }

    //- T6
    // Assert that the mutex is unlocked
    if (mutex_tag_t6 != INVALID_HANDLE) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert that the shared tag was set by Thread C
    if (shared_tag_t6 != TEST_0004_THREAD_C_TAG) {
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

//<editor-fold>

//<Editor fold desc="test 0005: Mutex multiple threads different priority">

#define TEST_0005_THREAD_A_TAG 0xA
#define TEST_0005_THREAD_A_LOCK_DELAY_MS 100
#define TEST_0005_THREAD_A_UNLOCK_DELAY_MS 500
#define TEST_0005_THREAD_A_PRIORITY 0x20

#define TEST_0005_THREAD_B_TAG 0xB
#define TEST_0005_THREAD_B_LOCK_DELAY_MS 200
#define TEST_0005_THREAD_B_UNLOCK_DELAY_MS 100
#define TEST_0005_THREAD_B_PRIORITY 0x2C

#define TEST_0005_THREAD_C_TAG 0xC
#define TEST_0005_THREAD_C_LOCK_DELAY_MS 300
#define TEST_0005_THREAD_C_UNLOCK_DELAY_MS 100
#define TEST_0005_THREAD_C_PRIORITY (TEST_0005_THREAD_B_PRIORITY - 1) // Higher priority than Thread B


static Mutex g_test_0005_mutex;
static int64_t g_test_0005_shared_tag = -1;

typedef struct {
    int64_t tag; ///< The tag to set the shared variable to.
    int64_t lock_delay_ms; ///< The delay in milliseconds before locking the mutex and setting the shared variable.
    int64_t unlock_delay_ms; ///< The delay in milliseconds before unlocking the mutex.
} Test0005_ThreadArgs;

/**
* Thread function for Test #0005
*/
void test_0005_thread_func(void *arg) {
    const Test0005_ThreadArgs *args = arg;

    threadSleepMs(args->lock_delay_ms);
    mutexLock(&g_test_0005_mutex);

    g_test_0005_shared_tag = args->tag;

    threadSleepMs(args->unlock_delay_ms);
    mutexUnlock(&g_test_0005_mutex);
}

/**
* This test creates multiple threads that each set a shared variable to their thread number.
* The mutex locks DO overlap, so the shared variable should be set to the thread number of the
* last thread to lock the mutex.
*
* Different priorities are used to test the priority inheritance mechanism.
*/
uint32_t test_0005_mutex_multiple_threads_different_priority(void) {
    Result rc = 0;

    //* Given
    // Initialize the test global mutex
    mutexInit(&g_test_0005_mutex);

    // Create threads
    Thread thread_a;
    Test0005_ThreadArgs thread_a_args = {
        .tag = TEST_0005_THREAD_A_TAG,
        .lock_delay_ms = TEST_0005_THREAD_A_LOCK_DELAY_MS,
        .unlock_delay_ms = TEST_0005_THREAD_A_UNLOCK_DELAY_MS
    };

    Thread thread_b;
    Test0005_ThreadArgs thread_b_args = {
        .tag = TEST_0005_THREAD_B_TAG,
        .lock_delay_ms = TEST_0005_THREAD_B_LOCK_DELAY_MS,
        .unlock_delay_ms = TEST_0005_THREAD_B_UNLOCK_DELAY_MS
    };

    Thread thread_c;
    Test0003_ThreadArgs thread_c_args = {
        .tag = TEST_0005_THREAD_C_TAG,
        .lock_delay_ms = TEST_0005_THREAD_C_LOCK_DELAY_MS,
        .unlock_delay_ms = TEST_0005_THREAD_C_UNLOCK_DELAY_MS
    };

    rc = threadCreate(&thread_a, test_0005_thread_func, &thread_a_args, NULL, 0x10000, TEST_0005_THREAD_A_PRIORITY, -2);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    rc = threadCreate(&thread_b, test_0005_thread_func, &thread_b_args, NULL, 0x10000, TEST_0005_THREAD_B_PRIORITY, -2);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    rc = threadCreate(&thread_c, test_0005_thread_func, &thread_c_args, NULL, 0x10000, TEST_0005_THREAD_C_PRIORITY, -2);
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

    // T1: Wait for Thread A to lock the mutex, and set the shared tag
    const int64_t t1 = t0 + TEST_0005_THREAD_A_LOCK_DELAY_MS + 10; // t0 + 100ms (+ 10ms)
    threadSleepMs(t1 - t0);

    const uint32_t mutex_tag_t1 = g_test_0005_mutex;
    const uint64_t shared_tag_t1 = g_test_0005_shared_tag; // Should be TEST_0005_THREAD_A_TAG

    // T2: Wait for Thread B to try to lock the mutex, mutex should be locked by Thread A and marked as contended
    const int64_t t2 = t0 + TEST_0005_THREAD_B_LOCK_DELAY_MS + 10; // t0 + 200ms (+ 10ms)
    threadSleepMs(t2 - t1);

    const uint32_t mutex_tag_t2 = g_test_0005_mutex;
    const uint64_t shared_tag_t2 = g_test_0005_shared_tag; // Should be TEST_0005_THREAD_A_TAG

    // T3: Wait for Thread C to try to lock the mutex, mutex should be locked by Thread A and marked as contended
    const int64_t t3 = t0 + TEST_0005_THREAD_C_LOCK_DELAY_MS + 10; // t0 + 300ms (+ 10ms)
    threadSleepMs(t3 - t2);

    const uint32_t mutex_tag_t3 = g_test_0005_mutex;
    const uint64_t shared_tag_t3 = g_test_0005_shared_tag; // Should be TEST_0005_THREAD_A_TAG

    // T4: Wait for Thread A to unlock the mutex, and Thread C to lock the mutex and set the shared tag
    const int64_t t4 = t1 + TEST_0005_THREAD_A_UNLOCK_DELAY_MS + 10; // t1 + 500ms (+ 10ms)
    threadSleepMs(t4 - t3);

    const uint32_t mutex_tag_t4 = g_test_0005_mutex;
    const uint64_t shared_tag_t4 = g_test_0005_shared_tag; // Should be TEST_0005_THREAD_C_TAG

    // T5: Wait for Thread C to unlock the mutex, and Thread B to lock the mutex and set the shared tag
    const int64_t t5 = t4 + TEST_0005_THREAD_C_UNLOCK_DELAY_MS + 10; // t4 + 100ms (+ 10ms)
    threadSleepMs(t5 - t4);

    const uint32_t mutex_tag_t5 = g_test_0005_mutex;
    const uint64_t shared_tag_t5 = g_test_0005_shared_tag; // Should be TEST_0005_THREAD_B_TAG

    // T6: Wait for Thread B to unlock the mutex
    const int64_t t6 = t5 + TEST_0005_THREAD_B_UNLOCK_DELAY_MS + 10; // t5 + 100ms (+ 10ms)
    threadSleepMs(t6 - t5);

    const uint32_t mutex_tag_t6 = g_test_0005_mutex;
    const uint64_t shared_tag_t6 = g_test_0005_shared_tag; // Should be TEST_0005_THREAD_B_TAG

    //* Then
    //- T1: Thread A locks the mutex
    // Assert that the mutex is locked (by Thread A), and there are no waiters
    if (!(mutex_tag_t1 != INVALID_HANDLE && (mutex_tag_t1 & HANDLE_WAIT_MASK) == 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert that the shared tag is set by Thread A
    if (shared_tag_t1 != TEST_0005_THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //- T2: Thread B tries to lock the mutex
    // Assert that the mutex is locked (by Thread A), and there are waiters (Thread B)
    if (!(mutex_tag_t2 != INVALID_HANDLE && (mutex_tag_t2 & HANDLE_WAIT_MASK) != 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert that the shared tag was set by Thread A
    if (shared_tag_t2 != TEST_0005_THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //- T3: Thread C tries to lock the mutex
    // Assert that the mutex is locked (by Thread B), and there are waiters (Threads B and C)
    if (!(mutex_tag_t3 != INVALID_HANDLE && (mutex_tag_t3 & HANDLE_WAIT_MASK) != 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert that the shared tag was set by Thread A
    if (shared_tag_t3 != TEST_0005_THREAD_A_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //- T4: Thread A unlocks the mutex, Thread C locks the mutex
    // Assert that the mutex is locked (by Thread C), and there are waiters (Thread B)
    if (!(mutex_tag_t4 != INVALID_HANDLE && (mutex_tag_t4 & HANDLE_WAIT_MASK) != 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert that the shared tag was set by Thread C
    if (shared_tag_t4 != TEST_0005_THREAD_C_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //- T5
    // Assert that the mutex is locked (by Thread B), and there are no waiters
    if (!(mutex_tag_t5 != INVALID_HANDLE && (mutex_tag_t5 & HANDLE_WAIT_MASK) == 0)) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert that the shared tag was set by Thread B
    if (shared_tag_t5 != TEST_0005_THREAD_B_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //- T6
    // Assert that the mutex is unlocked
    if (mutex_tag_t6 != INVALID_HANDLE) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Assert that the shared tag was set by Thread B
    if (shared_tag_t6 != TEST_0005_THREAD_B_TAG) {
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

//<editor-fold>
