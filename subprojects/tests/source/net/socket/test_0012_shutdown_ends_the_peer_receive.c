#include <stdbool.h>
#include <sys/socket.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0012: shutdown ends the peer receive">

test_rc_t test_0012_shutdown_ends_the_peer_receive(void)
{
    //* Given
    // A connected pair with nothing in flight, so the only thing the peer can
    // observe is the shutdown itself.
    int client = -1;
    int server = -1;
    if (net_connected_pair(NET_TEST_PORT, &client, &server) != 0) {
        return TEST_SETUP_FAILED;
    }

    //* When
    // The sending half is shut down, and the peer reads.
    const int closed = shutdown(client, SHUT_WR);

    char buf[8];
    const ssize_t got = recv(server, buf, sizeof(buf), 0);

    //* Then
    // The end of the stream is a zero byte count rather than a failure —
    // reporting it as an error would make every correct reader treat a normal
    // close as a fault.
    const bool correct = closed == 0 && got == 0;

    net_close(client);
    net_close(server);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
