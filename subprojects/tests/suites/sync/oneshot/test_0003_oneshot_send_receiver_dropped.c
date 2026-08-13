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
#define SEND_DELAY_MS 100

static NxSyncOneshotReceiver* g_receiver = NULL;

/**
 * Thread function: drops receiver before main thread sends.
 */
static void dropper_thread_func(void *arg) {
    threadSleepMs(DROP_DELAY_MS);

    // Drop receiver - this should cause subsequent send to fail
    __nx_std_sync__oneshot_receiver_free(g_receiver);
}

/**
 * Test that send fails when receiver is already dropped.
 * - Dropper thread frees receiver after delay
 * - Main thread waits longer, then calls send
 * - Verify send returns -1 (failure)
 */
test_rc_t test_0003_oneshot_send_receiver_dropped(void) {
    Result rc = 0;

    //* Given
    // Create oneshot channel
    NxSyncOneshotSender* sender = NULL;
    __nx_std_sync__oneshot_create(&sender, &g_receiver);

    // Create dropper thread
    Thread dropper_thread;
    rc = threadCreate(&dropper_thread, dropper_thread_func, NULL, NULL, 0x10000, 0x2C, -2);
    if (R_FAILED(rc)) {
        __nx_std_sync__oneshot_sender_free(sender);
        __nx_std_sync__oneshot_receiver_free(g_receiver);
        return rc;
    }

    //* When
    // Start dropper thread
    rc = threadStart(&dropper_thread);
    if (R_FAILED(rc)) {
        threadClose(&dropper_thread);
        __nx_std_sync__oneshot_sender_free(sender);
        __nx_std_sync__oneshot_receiver_free(g_receiver);
        return rc;
    }

    // Wait for dropper thread to drop the receiver
    threadSleepMs(SEND_DELAY_MS);

    // Try to send (should fail since receiver is dropped)
    void* value = (void*)(uintptr_t)0xDEADBEEF;
    int32_t send_rc = __nx_std_sync__oneshot_send(sender, value);

    //* Then
    // Verify send failed (receiver was dropped)
    if (send_rc != -1) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

test_cleanup:
    threadWaitForExit(&dropper_thread);
    threadClose(&dropper_thread);

    return rc;
}
