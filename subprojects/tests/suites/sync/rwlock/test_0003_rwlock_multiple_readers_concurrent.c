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

#define NUM_READERS 4
#define READ_DELAY_MS 100

static RwLock g_rwlock;
static Mutex g_mutex;
static uint32_t g_active_readers = 0;
static uint32_t g_max_concurrent_readers = 0;
static uint32_t g_completed_readers = 0;

/**
 * Thread function for Test #0003
 */
static void reader_thread_func(void *arg) {
    // Acquire read lock
    rwlockReadLock(&g_rwlock);
    
    // Update active readers count using mutex for thread safety
    mutexLock(&g_mutex);
    g_active_readers++;
    if (g_active_readers > g_max_concurrent_readers) {
        g_max_concurrent_readers = g_active_readers;
    }
    mutexUnlock(&g_mutex);
    
    // Do some read work
    threadSleepMs(READ_DELAY_MS);
    
    // Update counters using mutex for thread safety
    mutexLock(&g_mutex);
    g_active_readers--;
    g_completed_readers++;
    mutexUnlock(&g_mutex);
    
    // Release read lock
    rwlockReadUnlock(&g_rwlock);
}

/**
 * Test multiple readers can acquire read locks concurrently.
 */
test_rc_t test_0003_rwlock_multiple_readers_concurrent(void) {
    Result rc = 0;

    //* Given
    // Initialize the test global rwlock and mutex
    rwlockInit(&g_rwlock);
    mutexInit(&g_mutex);

    // Create reader threads
    Thread threads[NUM_READERS];
    for (int i = 0; i < NUM_READERS; i++) {
        rc = threadCreate(&threads[i], reader_thread_func, NULL, NULL, 0x10000, 0x2C, -2);
        if (R_FAILED(rc)) {
            goto test_cleanup;
        }
    }

    //* When
    // Start all threads simultaneously
    for (int i = 0; i < NUM_READERS; i++) {
        rc = threadStart(&threads[i]);
        if (R_FAILED(rc)) {
            goto test_cleanup;
        }
    }
    
    const int64_t t0 = 0;

    // T1: Sleep briefly to allow threads to acquire read locks
    const int64_t t1 = t0 + 10; // t0 + 10ms
    threadSleepMs(t1 - t0);

    // Check concurrent readers
    mutexLock(&g_mutex);
    const uint32_t active_readers_t1 = g_active_readers;
    const uint32_t completed_readers_t1 = g_completed_readers;
    mutexUnlock(&g_mutex);
    
    // T2: Wait for all readers to complete
    const int64_t t2 = t1 + READ_DELAY_MS + 10; // t1 + 100ms + 10ms
    threadSleepMs(t2 - t1);

    mutexLock(&g_mutex);
    const uint32_t active_readers_t2 = g_active_readers;
    const uint32_t completed_readers_t2 = g_completed_readers;
    const uint32_t max_concurrent_readers = g_max_concurrent_readers;
    mutexUnlock(&g_mutex);

    //* Then
    // - T1
    // Assert that all readers are active concurrently
    if (active_readers_t1 != NUM_READERS) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }
    
    // Assert that no readers have completed yet
    if (completed_readers_t1 != 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T2
    // Assert that no readers are active after completion
    if (active_readers_t2 != 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }
    
    // Assert that all readers have completed
    if (completed_readers_t2 != NUM_READERS) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }
    
    // Assert that all readers were concurrent at some point
    if (max_concurrent_readers != NUM_READERS) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //* Clean-up
test_cleanup:
    for (int i = 0; i < NUM_READERS; i++) {
        threadWaitForExit(&threads[i]);
        threadClose(&threads[i]);
    }

    return rc;
} 
