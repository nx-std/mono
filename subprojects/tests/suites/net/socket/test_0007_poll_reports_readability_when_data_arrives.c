#include <poll.h>
#include <stdbool.h>
#include <sys/socket.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0007: poll reports readability when data arrives">

test_rc_t test_0007_poll_reports_readability_when_data_arrives(void)
{
    //* Given
    // A connected pair with nothing sent yet, so the receiving end starts out
    // not readable.
    int client = -1;
    int server = -1;
    if (net_connected_pair(NET_TEST_PORT, &client, &server) != 0) {
        return TEST_SETUP_FAILED;
    }

    struct pollfd entry;
    entry.fd = server;
    entry.events = POLLIN;
    entry.revents = 0;

    //* When
    // The same entry is polled either side of a send. `poll` carries this
    // process's descriptor numbers and the service understands only its own, so
    // each entry is renumbered on the way in and matched back on the way out.
    const int idle = poll(&entry, 1, 0);
    const short idle_revents = entry.revents;

    const ssize_t sent = send(client, "x", 1, 0);

    entry.revents = 0;
    // A timeout rather than zero: the send is accepted locally before loopback
    // delivery makes the peer readable.
    const int ready = poll(&entry, 1, 1000);
    const short ready_revents = entry.revents;

    //* Then
    // Not readable first and readable second, for the same entry — an
    // implementation that lost the correspondence could report the second but
    // not the first.
    const bool correct = idle == 0
        && (idle_revents & POLLIN) == 0
        && sent == 1
        && ready == 1
        && (ready_revents & POLLIN) != 0;

    net_close(client);
    net_close(server);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
