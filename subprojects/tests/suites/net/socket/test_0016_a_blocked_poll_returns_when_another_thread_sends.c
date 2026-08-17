#include <poll.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <sys/socket.h>
#include <threads.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0016: a blocked poll returns when another thread sends">

/** How long the sender waits, so the poll is blocked before the send lands. */
#define SEND_DELAY_MS 200

/** How long the poll would wait if nothing ever made the socket ready. */
#define POLL_TIMEOUT_MS 5000

/**
 * @brief How soon the poll has to return for the send to be what returned it.
 *
 * Between the delay and the timeout, and far from both: a poll that noticed the
 * send comes back near the delay, and one that only looked when its timeout
 * expired comes back near the timeout.
 */
#define WOKEN_LIMIT_MS 2000

/**
 * @brief What the sending thread is given, and what it reports back.
 */
typedef struct {
    /** The socket to send on. */
    int fd;
    /** Zero once the byte has been sent, non-zero if it never was. */
    int rc;
} Sender;

/**
 * @brief Sleeps the current thread for the given number of milliseconds.
 */
static inline void threadSleepMs(int64_t ms) {
    svcSleepThread(ms * 1000000);
}

/**
 * Sends one byte, after a delay long enough for the other thread to have
 * reached its poll and blocked there.
 */
static int send_thread(void* arg) {
    Sender* sender = (Sender*)arg;
    threadSleepMs(SEND_DELAY_MS);
    sender->rc = send(sender->fd, "x", 1, 0) == 1 ? 0 : -1;
    return 0;
}

test_rc_t test_0016_a_blocked_poll_returns_when_another_thread_sends(void)
{
    //* Given
    // A connected pair with nothing sent yet, and a second thread that will
    // send on it shortly. Nothing else can make the watched end readable, so
    // the send is the only thing the poll can be returning for.
    int client = -1;
    int server = -1;
    if (net_connected_pair(NET_TEST_PORT, &client, &server) != 0) {
        return TEST_SETUP_FAILED;
    }

    Sender sender = { .fd = client, .rc = -1 };
    thrd_t thread;
    if (thrd_create(&thread, send_thread, &sender) != thrd_success) {
        net_close(client);
        net_close(server);
        return TEST_SETUP_FAILED;
    }

    struct pollfd entry;
    entry.fd = server;
    entry.events = POLLIN;
    entry.revents = 0;

    const uint64_t started = armGetSystemTick();

    //* When
    const int ready = poll(&entry, 1, POLL_TIMEOUT_MS);

    //* Then
    // How long it took is the assertion that matters. The readiness alone would
    // also be reported by a poll that slept out its whole timeout and only then
    // looked, and a wait that cannot be ended early is one an event loop cannot
    // be woken out of.
    const uint64_t elapsed_ms = armTicksToNs(armGetSystemTick() - started) / 1000000;
    thrd_join(thread, NULL);

    const bool correct = ready == 1
        && (entry.revents & POLLIN) != 0
        && elapsed_ms < WOKEN_LIMIT_MS
        && sender.rc == 0;

    net_close(client);
    net_close(server);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
