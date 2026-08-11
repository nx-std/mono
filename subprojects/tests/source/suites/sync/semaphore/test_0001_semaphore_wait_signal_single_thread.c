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

static Semaphore g_semaphore;
static bool g_task_completed = false;

/**
 * Thread function for Test #0001
 */
static void thread_func(void *arg) {
    // Wait on the semaphore
    semaphoreWait(&g_semaphore);
    
    // Set the task completed flag
    g_task_completed = true;

    // Signal the semaphore again
    semaphoreSignal(&g_semaphore);
}

/**
 * Test semaphore wait and signal in a single thread.
 */
test_rc_t test_0001_semaphore_wait_signal_single_thread(void) {
    Result rc = 0;

    //* Given
    // Initialize the test global semaphore with count 0
    semaphoreInit(&g_semaphore, 0);

    // Create a thread
    Thread thread;
    rc = threadCreate(&thread, thread_func, NULL, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }

    //* When
    // Start the thread
    rc = threadStart(&thread);
    if (R_FAILED(rc)) {
        goto test_cleanup;
    }


    const int64_t t0 = 0;

    // T1: Sleep briefly to ensure thread is waiting on semaphore
    const int64_t t1 = t0 + 10; // t0 + 10ms
    threadSleepMs(t1 - t0);
    
    // Check that task is not completed yet (thread should be blocked)
    const bool task_completed_t1 = g_task_completed;
    
    // Signal the semaphore to unblock the thread
    semaphoreSignal(&g_semaphore);
    
    // T2: Sleep briefly to allow thread to complete its work
    const int64_t t2 = t1 + 10; // t1 + 10ms
    threadSleepMs(t2 - t1);
    
    // Check that task is now completed
    const bool task_completed_t2 = g_task_completed;
    
    // Wait on the semaphore that the thread should have signaled
    semaphoreWait(&g_semaphore);
    
    //* Then
    // - T1
    // Assert that the task was not completed before signaling
    if (task_completed_t1) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }
    
    // - T2
    // Assert that the task was completed after signaling
    if (!task_completed_t2) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //* Clean-up
test_cleanup:
    threadWaitForExit(&thread);
    threadClose(&thread);

    return rc;
} 
