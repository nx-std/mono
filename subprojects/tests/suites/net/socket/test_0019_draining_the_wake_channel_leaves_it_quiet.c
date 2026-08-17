#include <stdbool.h>
#include <sys/socket.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0019: draining the wake channel leaves it quiet">

/** How many wakes are sent before anything reads the channel. */
#define WAKES_SENT 3

/**
 * @brief Reads the channel until there is nothing left in it.
 *
 * What the selector does with the channel before reporting a wake, and the
 * reason its receiving end is non-blocking: on a blocking socket the receive
 * that finds the channel empty is where this loop would stop and stay.
 *
 * Returns how many wakes it took out.
 */
static int drain_channel(int fd) {
    int taken = 0;
    for (;;) {
        char discarded;
        if (recv(fd, &discarded, sizeof(discarded), 0) != 1) {
            return taken;
        }
        taken++;
    }
}

test_rc_t test_0019_draining_the_wake_channel_leaves_it_quiet(void)
{
    //* Given
    // A channel with several wakes in it and nothing having read them. A wait
    // reports this channel readable the same way it reports any other socket,
    // so a wake left behind is one that reports the channel ready all over
    // again on the wait after this one.
    int receiver = -1;
    int sender = -1;
    if (net_wake_channel(&receiver, &sender) != 0) {
        return TEST_SETUP_FAILED;
    }

    for (int sent = 0; sent < WAKES_SENT; sent++) {
        if (send(sender, "\1", 1, 0) != 1) {
            net_close(sender);
            net_close(receiver);
            return TEST_SETUP_FAILED;
        }
    }

    if (!net_is_readable(receiver)) {
        net_close(sender);
        net_close(receiver);
        return TEST_SETUP_FAILED;
    }

    //* When
    const int drained = drain_channel(receiver);

    //* Then
    const bool correct = drained == WAKES_SENT && !net_is_readable(receiver);

    net_close(sender);
    net_close(receiver);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
