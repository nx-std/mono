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

#define DROP_DELAY_MS 50

static NxSyncOneshotSender* g_sender = NULL;

/**
 * Thread function: drops sender without sending a value.
 */
static void dropper_thread_func(void *arg) {
    threadSleepMs(DROP_DELAY_MS);

    // Drop sender without sending - this should wake up the receiver
    __nx_std_sync__oneshot_sender_free(g_sender);
}

/**
 * Test that recv fails when sender is dropped without sending.
 * - Dropper thread frees sender after delay
 * - Main thread blocks on recv, then wakes when sender dropped
 * - Verify recv returns -1 (failure)
 */
test_rc_t test_0002_oneshot_recv_sender_dropped(void) {
    Result rc = 0;

    //* Given
    // Create oneshot channel
    NxSyncOneshotReceiver* receiver = NULL;
    __nx_std_sync__oneshot_create(&g_sender, &receiver);

    // Create dropper thread
    Thread dropper_thread;
    rc = threadCreate(&dropper_thread, dropper_thread_func, NULL, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        __nx_std_sync__oneshot_sender_free(g_sender);
        __nx_std_sync__oneshot_receiver_free(receiver);
        return rc;
    }

    //* When
    // Start dropper thread
    rc = threadStart(&dropper_thread);
    if (R_FAILED(rc)) {
        threadClose(&dropper_thread);
        __nx_std_sync__oneshot_sender_free(g_sender);
        __nx_std_sync__oneshot_receiver_free(receiver);
        return rc;
    }

    // Receive value (blocks until sender sends or is dropped)
    void* received_value = NULL;
    int32_t recv_rc = __nx_std_sync__oneshot_recv(receiver, &received_value);

    //* Then
    // Verify recv failed (sender was dropped without sending)
    if (recv_rc != -1) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

test_cleanup:
    threadWaitForExit(&dropper_thread);
    threadClose(&dropper_thread);

    return rc;
}
