#include <poll.h>
#include <stdbool.h>
#include <stdint.h>
#include <sys/socket.h>
#include <threads.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0018: a self-addressed datagram channel ends a blocked poll">

/** How long the waking thread waits, so the poll is blocked before it sends. */
#define WAKE_DELAY_MS 200

/** How long the poll would wait if nothing ever woke it. */
#define WAKE_POLL_TIMEOUT_MS 5000

/**
 * @brief How soon the poll has to return for the wake to be what returned it.
 *
 * Between the delay and the timeout, and far from both, as in test 0016.
 */
#define WAKE_LIMIT_MS 2000

/**
 * @brief What the waking thread is given, and what it reports back.
 */
typedef struct {
    /** The sending end of the channel. */
    int fd;
    /** Zero once the wake has been sent, non-zero if it never was. */
    int rc;
} Waking;

/**
 * @brief Sleeps the current thread for the given number of milliseconds.
 */
static inline void wakeSleepMs(int64_t ms) {
    svcSleepThread(ms * 1000000);
}

/**
 * Sends one wake, after a delay long enough for the other thread to have
 * reached its poll and blocked there.
 */
static int wake_thread(void* arg) {
    Waking* waking = (Waking*)arg;
    wakeSleepMs(WAKE_DELAY_MS);
    waking->rc = send(waking->fd, "\1", 1, 0) == 1 ? 0 : -1;
    return 0;
}

test_rc_t test_0018_a_self_addressed_datagram_channel_ends_a_blocked_poll(void)
{
    //* Given
    // The wake channel, and a thread that will send on it once the wait below
    // has had time to block. Nothing else can make the watched end readable, so
    // the wake is the only thing the poll can be returning for.
    int receiver = -1;
    int sender = -1;
    if (net_wake_channel(&receiver, &sender) != 0) {
        return TEST_SETUP_FAILED;
    }

    Waking waking = { .fd = sender, .rc = -1 };
    thrd_t thread;
    if (thrd_create(&thread, wake_thread, &waking) != thrd_success) {
        net_close(sender);
        net_close(receiver);
        return TEST_SETUP_FAILED;
    }

    struct pollfd entry;
    entry.fd = receiver;
    entry.events = POLLIN;
    entry.revents = 0;

    const uint64_t started = armGetSystemTick();

    //* When
    const int ready = poll(&entry, 1, WAKE_POLL_TIMEOUT_MS);

    //* Then
    // How long it took is the assertion that matters, as in test 0016: a wait
    // that only returns when its timeout expires is not one a waker ended.
    const uint64_t elapsed_ms = armTicksToNs(armGetSystemTick() - started) / 1000000;
    thrd_join(thread, NULL);

    const bool correct = ready == 1
        && (entry.revents & POLLIN) != 0
        && elapsed_ms < WAKE_LIMIT_MS
        && waking.rc == 0;

    net_close(sender);
    net_close(receiver);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
