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

#define NUM_THREADS 4
#define SEMAPHORE_INITIAL_COUNT 2
#define WORK_DELAY_MS 100

static Semaphore g_semaphore;
static Mutex g_mutex;
static uint32_t g_active_threads = 0;
static uint32_t g_completed_threads = 0;

/**
 * Thread function for Test #0002
 */
static void thread_func(void *arg) {
    // Wait on the semaphore
    semaphoreWait(&g_semaphore);
    
    // Increment active threads count using mutex for thread safety
    mutexLock(&g_mutex);
    g_active_threads++;
    mutexUnlock(&g_mutex);
    
    // Do some work
    threadSleepMs(WORK_DELAY_MS);
    
    // Update counters using mutex for thread safety
    mutexLock(&g_mutex);
    g_active_threads--;
    g_completed_threads++;
    mutexUnlock(&g_mutex);
    
    // Signal the semaphore to allow another thread to proceed
    semaphoreSignal(&g_semaphore);
}

/**
 * This test creates multiple threads that wait on a semaphore with an initial count.
 * Each thread decrements the semaphore count and performs its work.
 */
test_rc_t test_0002_semaphore_multiple_threads_initial_count(void) {
    Result rc = 0;

    //* Given
    // Initialize the test global semaphore with initial count and mutex
    semaphoreInit(&g_semaphore, SEMAPHORE_INITIAL_COUNT);
    mutexInit(&g_mutex);

    // Create threads
    Thread threads[NUM_THREADS];
    for (int i = 0; i < NUM_THREADS; i++) {
        rc = threadCreate(&threads[i], thread_func, NULL, NULL, 0x10000, 0x2C, -2);
        if (R_FAILED(rc)) {
            goto test_cleanup;
        }
    }

    //* When
    // Start all threads
    for (int i = 0; i < NUM_THREADS; i++) {
        rc = threadStart(&threads[i]);
        if (R_FAILED(rc)) {
            goto test_cleanup;
        }
    }
    
    const int64_t t0 = 0;

    // T1: Sleep briefly to allow threads to start
    const int64_t t1 = t0 + 10; // t0 + 10ms
    threadSleepMs(t1 - t0);

    // Check initial active threads (should match initial semaphore count)
    mutexLock(&g_mutex);
    const uint32_t active_threads_t1 = g_active_threads;
    const uint32_t completed_threads_t1 = g_completed_threads;
    mutexUnlock(&g_mutex);
    
    // T2: Wait for half the threads to complete
    const int64_t t2 = t1 + WORK_DELAY_MS + 10; // t1 + 100ms + 10ms
    threadSleepMs(t2 - t1);

    mutexLock(&g_mutex);
    const uint32_t active_threads_t2 = g_active_threads;
    const uint32_t completed_threads_t2 = g_completed_threads;
    mutexUnlock(&g_mutex);

    // T3: Wait for the remaining threads to complete
    const int64_t t3 = t1 + 2 * WORK_DELAY_MS + 10; // t1 + 200ms + 10ms
    threadSleepMs(t3 - t2);
    
    mutexLock(&g_mutex);
    const uint32_t active_threads_t3 = g_active_threads;
    const uint32_t completed_threads_t3 = g_completed_threads;
    mutexUnlock(&g_mutex);

    //* Then
    // - T1
    // Assert that initial active threads matches the semaphore count
    if (active_threads_t1 != SEMAPHORE_INITIAL_COUNT || completed_threads_t1 != 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T2
    // Assert that at the midpoint, we still have the same number of active threads
    // and some threads have completed
    if (active_threads_t2 != SEMAPHORE_INITIAL_COUNT || completed_threads_t2 == 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // - T3
    // Assert that all threads eventually completed
    if (active_threads_t3 != 0 || completed_threads_t3 != NUM_THREADS) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //* Clean-up
test_cleanup:
    for (int i = 0; i < NUM_THREADS; i++) {
        threadWaitForExit(&threads[i]);
        threadClose(&threads[i]);
    }

    return rc;
} 
