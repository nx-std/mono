#include <stdbool.h>
#include <sys/socket.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0013: nonblocking connect over loopback completes inline">

test_rc_t test_0013_nonblocking_connect_over_loopback_completes_inline(void)
{
    //* Given
    // A listener to connect to, and a non-blocking socket that has made no
    // other call yet, so the connect is the first thing the flag affects.
    const int listener = net_listen_loopback(NET_TEST_PORT);
    if (listener < 0) {
        return TEST_SETUP_FAILED;
    }

    const int client = net_nonblocking_socket();
    if (client < 0) {
        net_close(listener);
        return TEST_SETUP_FAILED;
    }

    struct sockaddr_in addr;
    net_loopback_addr(&addr, NET_TEST_PORT);

    //* When
    const int rc = connect(client, (struct sockaddr*)&addr, sizeof(addr));

    //* Then
    // Success rather than the `EINPROGRESS` a non-blocking connect reports
    // elsewhere: over loopback both ends are this console, so the connection is
    // made before the call returns and there is no in-flight state to wait out.
    // The socket carries no pending error either, which is what says the
    // success is the whole answer and not one half of it.
    //
    // A caller must not read this as the general rule. It is what loopback
    // does; a connect to a peer over a network has to travel, and the state
    // this console skips is the one it would be left in.
    const bool correct = rc == 0 && net_pending_error(client) == 0;

    net_close(client);
    net_close(listener);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
