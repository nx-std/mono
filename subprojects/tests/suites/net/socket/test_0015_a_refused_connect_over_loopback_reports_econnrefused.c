#include <stdbool.h>
#include <sys/socket.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0015: a refused connect over loopback reports econnrefused">

/**
 * @brief A loopback port this suite never binds.
 *
 * One past the port the rest of the suite uses, so a connect to it is refused
 * rather than answered by a listener an earlier test left open.
 */
#define NET_UNUSED_PORT (NET_TEST_PORT + 1)

test_rc_t test_0015_a_refused_connect_over_loopback_reports_econnrefused(void)
{
    //* Given
    // A non-blocking socket and a port nothing is listening on.
    const int client = net_nonblocking_socket();
    if (client < 0) {
        return TEST_SETUP_FAILED;
    }

    struct sockaddr_in addr;
    net_loopback_addr(&addr, NET_UNUSED_PORT);

    //* When
    const int rc = connect(client, (struct sockaddr*)&addr, sizeof(addr));
    const int reported = errno;

    //* Then
    // The refusal arrives from the connect itself, the same way the success in
    // test 0013 does: over loopback there is nothing in flight to be told about
    // later. A caller written for a connect that reports its verdict through
    // the socket afterwards is not wrong here — it just never has a verdict to
    // collect, because the call already gave it one.
    const bool correct = rc < 0 && reported == ECONNREFUSED;

    net_close(client);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
