#include <errno.h>
#include <fcntl.h>
#include <stdbool.h>
#include <sys/socket.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0006: non-blocking accept on an idle listener reports EAGAIN">

test_rc_t test_0006_nonblocking_accept_on_an_idle_listener_reports_eagain(void)
{
    //* Given
    // A listening socket with nothing connecting to it, so an accept has
    // nothing to take off the queue.
    const int listener = net_listen_loopback(NET_TEST_PORT);
    if (listener < 0) {
        return TEST_SETUP_FAILED;
    }

    //* When
    // The flag is set through `fcntl`, which has to reach the service since it
    // belongs to the socket, then read back, then the accept is attempted.
    const int flags = fcntl(listener, F_GETFL, 0);
    const int set = (flags >= 0) ? fcntl(listener, F_SETFL, flags | O_NONBLOCK) : -1;
    const int read_back = fcntl(listener, F_GETFL, 0);

    errno = 0;
    const int accepted = accept(listener, NULL, NULL);
    const int accept_errno = errno;

    //* Then
    // The flag took, and the refusal arrived as `EAGAIN` — the service answers
    // in Linux's numbering, where the same condition is a different number, so
    // an untranslated code would surface here as something a caller polling for
    // readiness would never recognise.
    const bool correct = flags >= 0
        && set >= 0
        && (read_back & O_NONBLOCK) != 0
        && accepted < 0
        && (accept_errno == EAGAIN || accept_errno == EWOULDBLOCK);

    net_close(listener);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
