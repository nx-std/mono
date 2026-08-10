#include <netinet/in.h>
#include <stdbool.h>
#include <string.h>
#include <sys/socket.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0005: UDP loopback round trips a datagram">

#define PAYLOAD "a datagram"

test_rc_t test_0005_udp_loopback_round_trips_a_datagram(void)
{
    //* Given
    // A datagram socket bound to the loopback port, and another to send from.
    const int receiver = socket(AF_INET, SOCK_DGRAM, 0);
    if (receiver < 0) {
        return TEST_SETUP_FAILED;
    }

    struct sockaddr_in bound;
    net_loopback_addr(&bound, NET_TEST_PORT);
    if (bind(receiver, (struct sockaddr*)&bound, sizeof(bound)) != 0) {
        net_close(receiver);
        return TEST_SETUP_FAILED;
    }

    const int sender = socket(AF_INET, SOCK_DGRAM, 0);
    if (sender < 0) {
        net_close(receiver);
        return TEST_SETUP_FAILED;
    }

    //* When
    // The datagram goes out through `sendto` and comes back through `recvfrom`,
    // which is the one receive that reports an address. The sender is asked
    // what it was assigned only after sending, since that is when it is bound.
    const ssize_t sent =
        sendto(sender, PAYLOAD, sizeof(PAYLOAD), 0, (struct sockaddr*)&bound, sizeof(bound));

    struct sockaddr_in expected;
    memset(&expected, 0, sizeof(expected));
    socklen_t expected_len = sizeof(expected);
    const int named = getsockname(sender, (struct sockaddr*)&expected, &expected_len);

    char received[sizeof(PAYLOAD)];
    memset(received, 0, sizeof(received));
    struct sockaddr_in from;
    memset(&from, 0, sizeof(from));
    socklen_t from_len = sizeof(from);
    const ssize_t got =
        recvfrom(receiver, received, sizeof(received), 0, (struct sockaddr*)&from, &from_len);

    //* Then
    // The payload survived, and the reported sender is the sending socket
    // rather than a zeroed address that would pass an existence check alone.
    const bool correct = sent == (ssize_t)sizeof(PAYLOAD)
        && named == 0
        && got == (ssize_t)sizeof(PAYLOAD)
        && memcmp(PAYLOAD, received, sizeof(PAYLOAD)) == 0
        && from_len == sizeof(from)
        && from.sin_family == AF_INET
        && from.sin_port == expected.sin_port;

    net_close(sender);
    net_close(receiver);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
