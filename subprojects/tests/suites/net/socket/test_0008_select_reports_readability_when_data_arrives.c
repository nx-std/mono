#include <stdbool.h>
#include <sys/select.h>
#include <sys/socket.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0008: select reports readability when data arrives">

test_rc_t test_0008_select_reports_readability_when_data_arrives(void)
{
    //* Given
    // A connected pair with nothing sent yet, and a descriptor set naming only
    // the receiving end.
    int client = -1;
    int server = -1;
    if (net_connected_pair(NET_TEST_PORT, &client, &server) != 0) {
        return TEST_SETUP_FAILED;
    }

    //* When
    // The set is selected on either side of a send. `select` is not sent to the
    // service as a `select`: its bitmaps cannot survive renumbering, so it is
    // answered by a `poll` underneath and the sets rebuilt from the result.
    fd_set readable;
    struct timeval immediately = {.tv_sec = 0, .tv_usec = 0};
    FD_ZERO(&readable);
    FD_SET(server, &readable);

    const int idle = select(server + 1, &readable, NULL, NULL, &immediately);
    const bool idle_named_server = FD_ISSET(server, &readable) != 0;

    const ssize_t sent = send(client, "x", 1, 0);

    FD_ZERO(&readable);
    FD_SET(server, &readable);
    struct timeval within_a_second = {.tv_sec = 1, .tv_usec = 0};

    const int ready = select(server + 1, &readable, NULL, NULL, &within_a_second);
    const bool ready_named_server = FD_ISSET(server, &readable) != 0;
    const bool ready_named_client = FD_ISSET(client, &readable) != 0;

    //* Then
    // The returned set names the descriptor the caller asked about, and only
    // that one: the client end was never in the set going in.
    const bool correct = idle == 0
        && !idle_named_server
        && sent == 1
        && ready == 1
        && ready_named_server
        && !ready_named_client;

    net_close(client);
    net_close(server);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
