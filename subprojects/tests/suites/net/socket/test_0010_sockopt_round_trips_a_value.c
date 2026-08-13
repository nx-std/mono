#include <stdbool.h>
#include <sys/socket.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0010: socket options round trip a value">

test_rc_t test_0010_sockopt_round_trips_a_value(void)
{
    //* Given
    // A socket with no option set on it yet.
    const int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) {
        return TEST_SETUP_FAILED;
    }

    //* When
    // The option's bytes travel down and back, and the length coming back is
    // written into the caller's own variable.
    const int enabled = 1;
    const int written = setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &enabled, sizeof(enabled));

    int reported = 0;
    socklen_t len = sizeof(reported);
    const int read_back = getsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &reported, &len);

    //* Then
    // The option took, and the reported length describes what was written
    // rather than the option's own size — reporting more than the caller asked
    // for would have it read past the end of its own variable.
    const bool correct = written == 0
        && read_back == 0
        && len == sizeof(reported)
        && reported != 0;

    net_close(fd);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
