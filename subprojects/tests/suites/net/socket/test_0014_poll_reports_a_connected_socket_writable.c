#include <poll.h>
#include <stdbool.h>
#include <sys/socket.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0014: poll reports a connected socket writable">

/** How long to wait before giving up on a socket that should already be ready. */
#define WRITABLE_TIMEOUT_MS 5000

test_rc_t test_0014_poll_reports_a_connected_socket_writable(void)
{
    //* Given
    // A connected socket with an empty send buffer, which is the state a caller
    // watching for a chance to write starts from. The listener is never
    // accepted from: the backlog completes the handshake on its behalf.
    int client = -1;
    int server = -1;
    if (net_connected_pair(NET_TEST_PORT, &client, &server) != 0) {
        return TEST_SETUP_FAILED;
    }

    struct pollfd entry;
    entry.fd = client;
    entry.events = POLLOUT;
    entry.revents = 0;

    //* When
    const int ready = poll(&entry, 1, WRITABLE_TIMEOUT_MS);

    //* Then
    // Writability is the other half of what a wait reports, and the half every
    // other case here leaves untested. A caller that can only be told about
    // readable sockets has to send blind and find out afterwards.
    const bool correct = ready == 1 && (entry.revents & POLLOUT) != 0;

    net_close(client);
    net_close(server);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
