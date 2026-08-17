#include <poll.h>
#include <stdbool.h>
#include <sys/socket.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0017: poll accepts a set of many sockets">

/**
 * @brief How many sockets one wait is asked about.
 *
 * More than a single connection's worth and fewer than a server's: enough to
 * show the wait is not limited to the handful every other test in this suite
 * passes it, which is the question a caller watching a whole connection table
 * asks.
 */
#define POLL_SET_SIZE 32

test_rc_t test_0017_poll_accepts_a_set_of_many_sockets(void)
{
    //* Given
    // A set of unbound datagram sockets, none of which anything can send to, so
    // the wait has nothing to report about any of them and answers on the size
    // of the set alone. Datagram sockets because they need no peer: a set this
    // size of connected pairs would be measuring the connections instead.
    int fds[POLL_SET_SIZE];
    struct pollfd entries[POLL_SET_SIZE];
    int created = 0;

    while (created < POLL_SET_SIZE) {
        const int fd = socket(AF_INET, SOCK_DGRAM, 0);
        if (fd < 0) {
            break;
        }
        fds[created] = fd;
        entries[created].fd = fd;
        entries[created].events = POLLIN;
        entries[created].revents = 0;
        created++;
    }

    if (created < POLL_SET_SIZE) {
        for (int i = 0; i < created; i++) {
            net_close(fds[i]);
        }
        return TEST_SETUP_FAILED;
    }

    //* When
    const int ready = poll(entries, POLL_SET_SIZE, 0);

    //* Then
    // Nothing ready is the answer, and a count of zero is how it is spelled. A
    // set the wait will not accept is refused outright instead, which is the
    // outcome this test exists to rule out.
    const bool correct = ready == 0;

    for (int i = 0; i < POLL_SET_SIZE; i++) {
        net_close(fds[i]);
    }
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
