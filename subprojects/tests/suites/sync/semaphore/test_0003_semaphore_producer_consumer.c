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

#define NUM_PRODUCERS 2
#define NUM_CONSUMERS 3
#define BUFFER_SIZE 5
#define ITEMS_PER_PRODUCER 4
#define PRODUCER_DELAY_MS 30
#define CONSUMER_DELAY_MS 50
#define EXPECTED_PRODUCED (NUM_PRODUCERS * ITEMS_PER_PRODUCER)

static Semaphore g_empty_semaphore;  // Counts empty slots in buffer
static Semaphore g_full_semaphore;   // Counts filled slots in buffer
static Mutex g_buffer_mutex;         // Protects access to the buffer
static uint32_t g_buffer_count = 0;  // Number of items in buffer
static uint32_t g_produced_count = 0; // Total items produced
static uint32_t g_consumed_count = 0; // Total items consumed
static bool g_producers_done = false; // Flag to signal consumers to exit

/**
 * Producer thread function for Test #0003
 */
static void producer_thread_func(void *arg) {
    for (int i = 0; i < ITEMS_PER_PRODUCER; i++) {
        // Simulate production time
        threadSleepMs(PRODUCER_DELAY_MS);
        
        // Wait for an empty slot
        semaphoreWait(&g_empty_semaphore);
        
        // Lock the buffer, add an item to the buffer, and unlock the buffer
        mutexLock(&g_buffer_mutex);
        g_buffer_count++;
        g_produced_count++;
        mutexUnlock(&g_buffer_mutex);
        
        // Signal that a slot is filled
        semaphoreSignal(&g_full_semaphore);
    }
}

/**
 * Consumer thread function for Test #0003
 */
static void consumer_thread_func(void *arg) {
    while (true) {
        // Check if producers are done and buffer is empty
        mutexLock(&g_buffer_mutex);
        const bool should_exit = g_producers_done && (g_buffer_count == 0);
        mutexUnlock(&g_buffer_mutex);
        
        if (should_exit) {
            return;
        }
        
        // Try to get an item without blocking
        if (!semaphoreTryWait(&g_full_semaphore)) {
            // No items available, sleep briefly and try again
            threadSleepMs(10);
            continue;
        }
        
        // Lock the buffer
        mutexLock(&g_buffer_mutex);
        
        // Remove item from buffer
        if (g_buffer_count > 0) {
            g_buffer_count--;
            g_consumed_count++;
        }
        
        // Unlock the buffer
        mutexUnlock(&g_buffer_mutex);
        
        // Signal that a slot is empty
        semaphoreSignal(&g_empty_semaphore);
        
        // Simulate consumption time
        threadSleepMs(CONSUMER_DELAY_MS);
    }
}

/**
 * This test creates multiple producer and consumer threads.
 * Producer threads signal the semaphore, and consumer threads wait on it.
 */
test_rc_t test_0003_semaphore_producer_consumer(void) {
    Result rc = 0;

    //* Given
    // Initialize the semaphores and mutex
    semaphoreInit(&g_empty_semaphore, BUFFER_SIZE);
    semaphoreInit(&g_full_semaphore, 0);
    mutexInit(&g_buffer_mutex);

    // Create producer threads
    Thread producers[NUM_PRODUCERS];
    for (int i = 0; i < NUM_PRODUCERS; i++) {
        rc = threadCreate(&producers[i], producer_thread_func, NULL, NULL, 0x10000, 0x2C, -2);
        if (R_FAILED(rc)) {
            goto test_cleanup;
        }
    }
    
    // Create consumer threads
    Thread consumers[NUM_CONSUMERS];
    for (int i = 0; i < NUM_CONSUMERS; i++) {
        rc = threadCreate(&consumers[i], consumer_thread_func, NULL, NULL, 0x10000, 0x2C, -2);
        if (R_FAILED(rc)) {
            goto test_cleanup;
        }
    }

    //* When
    // Start all producer threads
    for (int i = 0; i < NUM_PRODUCERS; i++) {
        rc = threadStart(&producers[i]);
        if (R_FAILED(rc)) {
            goto test_cleanup;
        }
    }
    
    // Start all consumer threads
    for (int i = 0; i < NUM_CONSUMERS; i++) {
        rc = threadStart(&consumers[i]);
        if (R_FAILED(rc)) {
            goto test_cleanup;
        }
    }
    
    // T1: Wait for all producer threads to complete
    for (int i = 0; i < NUM_PRODUCERS; i++) {
        threadWaitForExit(&producers[i]);
    }
    
    // Signal consumers that producers are done
    mutexLock(&g_buffer_mutex);
    g_producers_done = true;
    mutexUnlock(&g_buffer_mutex);
    
    // T2: Wait for all consumer threads to complete
    for (int i = 0; i < NUM_CONSUMERS; i++) {
        threadWaitForExit(&consumers[i]);
    }
    
    mutexLock(&g_buffer_mutex);
    const uint32_t total_produced = g_produced_count;
    const uint32_t total_consumed = g_consumed_count;
    const uint32_t items_in_buffer = g_buffer_count;
    mutexUnlock(&g_buffer_mutex);

    //* Then
    // Assert that the expected number of items were produced
    if (total_produced != EXPECTED_PRODUCED) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }
    
    // Assert that all produced items were consumed
    if (total_consumed != EXPECTED_PRODUCED) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }
    
    // Assert that the buffer is empty
    if (items_in_buffer != 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    //* Clean-up
test_cleanup:
    for (int i = 0; i < NUM_PRODUCERS; i++) {
        threadWaitForExit(&producers[i]);
        threadClose(&producers[i]);
    }
    for (int i = 0; i < NUM_CONSUMERS; i++) {
        threadWaitForExit(&consumers[i]);
        threadClose(&consumers[i]);
    }

    return rc;
} 
