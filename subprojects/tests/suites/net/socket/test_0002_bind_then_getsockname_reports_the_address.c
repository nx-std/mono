#include <netinet/in.h>
#include <stdbool.h>
#include <string.h>
#include <sys/socket.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0002: bind then getsockname reports the address">

test_rc_t test_0002_bind_then_getsockname_reports_the_address(void)
{
    //* Given
    // A socket bound to the loopback port, so there is a known address for the
    // service to report back.
    const int fd = net_listen_loopback(NET_TEST_PORT);
    if (fd < 0) {
        return TEST_SETUP_FAILED;
    }

    //* When
    // The address travels back up into a caller's buffer, along with a length
    // written into the caller's own variable.
    struct sockaddr_in reported;
    memset(&reported, 0, sizeof(reported));
    socklen_t len = sizeof(reported);
    const int rc = getsockname(fd, (struct sockaddr*)&reported, &len);

    //* Then
    // The reported address is the one bound, and the reported length describes
    // it rather than whatever the buffer happened to be.
    const bool correct = rc == 0
        && len == sizeof(reported)
        && reported.sin_family == AF_INET
        && ntohs(reported.sin_port) == NET_TEST_PORT
        && ntohl(reported.sin_addr.s_addr) == INADDR_LOOPBACK;

    net_close(fd);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
