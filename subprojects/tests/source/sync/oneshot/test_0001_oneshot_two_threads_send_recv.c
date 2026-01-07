#include <stdint.h>
#include <stdbool.h>
#include <switch.h>

#include "../../harness.h"
#include "nx_sync_oneshot.h"

/**
 * @brief Sleeps the current thread for the given number of milliseconds.
 * @param ms The number of milliseconds to sleep.
 */
static inline void threadSleepMs(int64_t ms) {
    svcSleepThread(ms * 1000000);
}

#define SENDER_DELAY_MS 50
#define EXPECTED_VALUE 0xDEADBEEF

static NxSyncOneshotSender* g_sender = NULL;

/**
 * Sender thread function: sends a value on the oneshot channel.
 */
static void sender_thread_func(void *arg) {
    threadSleepMs(SENDER_DELAY_MS);

    void* value = (void*)(uintptr_t)EXPECTED_VALUE;
    __nx_std_sync__oneshot_send(g_sender, value);
}

/**
 * Test sending and receiving a value across two threads using a oneshot channel.
 * - Sender thread sends a value after a brief delay
 * - Main thread blocks on recv until value arrives
 * - Verify received value matches expected value
 */
test_rc_t test_0001_oneshot_two_threads_send_recv(void) {
    Result rc = 0;

    //* Given
    // Create oneshot channel
    NxSyncOneshotReceiver* receiver = NULL;
    __nx_std_sync__oneshot_create(&g_sender, &receiver);

    // Create sender thread
    Thread sender_thread;
    rc = threadCreate(&sender_thread, sender_thread_func, NULL, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        __nx_std_sync__oneshot_sender_free(g_sender);
        __nx_std_sync__oneshot_receiver_free(receiver);
        return rc;
    }

    //* When
    // Start sender thread
    rc = threadStart(&sender_thread);
    if (R_FAILED(rc)) {
        threadClose(&sender_thread);
        __nx_std_sync__oneshot_sender_free(g_sender);
        __nx_std_sync__oneshot_receiver_free(receiver);
        return rc;
    }

    // Receive value (blocks until sender sends)
    void* received_value = NULL;
    int32_t recv_rc = __nx_std_sync__oneshot_recv(receiver, &received_value);

    //* Then
    // Verify recv succeeded
    if (recv_rc != 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

    // Verify received value matches expected
    if ((uintptr_t)received_value != EXPECTED_VALUE) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

test_cleanup:
    threadWaitForExit(&sender_thread);
    threadClose(&sender_thread);

    return rc;
}
