#include <stdbool.h>
#include <string.h>
#include <unistd.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0004: read and write reach the socket">

#define PAYLOAD "written through the descriptor table"

test_rc_t test_0004_read_and_write_reach_the_socket(void)
{
    //* Given
    // A connected pair, to be driven by the two calls that have no
    // socket-specific C symbol behind them.
    int client = -1;
    int server = -1;
    if (net_connected_pair(NET_TEST_PORT, &client, &server) != 0) {
        return TEST_SETUP_FAILED;
    }

    //* When
    // `write` and `read` reach the socket only through the descriptor table,
    // which dispatches into the socket device. Nothing else here covers that
    // path, and it does nothing useful if the device was never registered.
    const ssize_t written = write(client, PAYLOAD, sizeof(PAYLOAD));

    char received[sizeof(PAYLOAD)];
    memset(received, 0, sizeof(received));
    const ssize_t got = read(server, received, sizeof(received));

    //* Then
    // The bytes made the trip, so the table dispatched both calls into the
    // socket rather than into whatever else might hold the descriptor.
    const bool correct = written == (ssize_t)sizeof(PAYLOAD)
        && got == (ssize_t)sizeof(PAYLOAD)
        && memcmp(PAYLOAD, received, sizeof(PAYLOAD)) == 0;

    net_close(client);
    net_close(server);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
