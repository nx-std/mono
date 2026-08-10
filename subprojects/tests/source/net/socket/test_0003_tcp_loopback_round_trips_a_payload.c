#include <stdbool.h>
#include <string.h>
#include <sys/socket.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0003: TCP loopback round trips a payload">

#define PAYLOAD "the quick brown fox jumps over the lazy dog"

test_rc_t test_0003_tcp_loopback_round_trips_a_payload(void)
{
    //* Given
    // A connected pair over loopback, both ends in this process, so the whole
    // server sequence — listen, connect, accept — has already succeeded.
    int client = -1;
    int server = -1;
    if (net_connected_pair(NET_TEST_PORT, &client, &server) != 0) {
        return TEST_SETUP_FAILED;
    }

    //* When
    // The payload goes out through `send` and comes back through `recv`.
    const ssize_t sent = send(client, PAYLOAD, sizeof(PAYLOAD), 0);

    char received[sizeof(PAYLOAD)];
    memset(received, 0, sizeof(received));
    const ssize_t got = recv(server, received, sizeof(received), 0);

    //* Then
    // What arrived is byte for byte what was sent — a length alone would pass
    // on a buffer handed to the service with the wrong bounds.
    const bool correct = sent == (ssize_t)sizeof(PAYLOAD)
        && got == (ssize_t)sizeof(PAYLOAD)
        && memcmp(PAYLOAD, received, sizeof(PAYLOAD)) == 0;

    net_close(client);
    net_close(server);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
