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


//<editor-fold desc="Test 0001: Mutex lock unlock single thread">

#define THREAD_TAG 42

static Mutex g_mutex;
static int64_t g_shared_tag = -1;

/**
 * Thread function for Test #0001
 */
static void thread_func(void *arg) {
    const int64_t num = (int64_t) arg;

    mutexLock(&g_mutex);
    g_shared_tag = num;
    mutexUnlock(&g_mutex);
}


/**
* Test mutex lock and unlock in a single thread.
*/
uint32_t test_0001_mutex_lock_unlock_single_thread(void) {
    Result rc = 0;

    //* Given
    // Initialize the test global mutex
    mutexInit(&g_mutex);

    // Create a thread
    Thread thread;
    rc = threadCreate(&thread, thread_func, (void *) THREAD_TAG, NULL, 0x10000, 0x2C, -2);
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

    uint64_t shared_tag = g_shared_tag;

    //* Then
    // Assert that the shared tag is set to THREAD_TAG
    if (shared_tag != THREAD_TAG) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

test_cleanup:
    threadWaitForExit(&thread);
    threadClose(&thread);

    return rc;
}

//</editor-fold>
