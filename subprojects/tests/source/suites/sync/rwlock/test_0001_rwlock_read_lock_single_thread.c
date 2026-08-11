#include <stdint.h>
#include <stdbool.h>

#include <switch.h>

#include "../../harness.h"

/**
* @brief Sleeps the current thread for the given number of milliseconds.
* @param ms The number of milliseconds to sleep.
*/
static inline void threadSleepMs(int64_t ms) {
    svcSleepThread(ms * 1000000);
}

#define THREAD_TAG 42

static RwLock g_rwlock;
static int64_t g_shared_tag = -1;

/**
 * Thread function for Test #0001
 */
static void thread_func(void *arg) {
    const int64_t num = (int64_t) arg;

    rwlockReadLock(&g_rwlock);
    g_shared_tag = num;
    rwlockReadUnlock(&g_rwlock);
}

/**
 * Test rwlock basic read lock functionality in a single thread.
 */
test_rc_t test_0001_rwlock_read_lock_single_thread(void) {
    Result rc = 0;

    //* Given
    // Initialize the test global rwlock
    rwlockInit(&g_rwlock);

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
