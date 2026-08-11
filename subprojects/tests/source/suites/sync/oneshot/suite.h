#pragma once

#include "../../harness.h"

/**
 * Test sending and receiving a value across two threads using a oneshot channel.
 * The sender thread sends a value, and the main thread receives and verifies it.
 */
test_rc_t test_0001_oneshot_two_threads_send_recv(void);

/**
 * Test that recv fails when sender is dropped without sending.
 */
test_rc_t test_0002_oneshot_recv_sender_dropped(void);

/**
 * Test that send fails when receiver is already dropped.
 */
test_rc_t test_0003_oneshot_send_receiver_dropped(void);

/**
 * Test suite for sync/oneshot.
 */
static void sync_oneshot_suite(void) {
    TEST_SUITE("sync/oneshot");

    TEST_CASE(
        "Test 0001: oneshot_two_threads_send_recv",
        test_0001_oneshot_two_threads_send_recv
    )
    TEST_CASE(
        "Test 0002: oneshot_recv_sender_dropped",
        test_0002_oneshot_recv_sender_dropped
    )
    TEST_CASE(
        "Test 0003: oneshot_send_receiver_dropped",
        test_0003_oneshot_send_receiver_dropped
    )
}
