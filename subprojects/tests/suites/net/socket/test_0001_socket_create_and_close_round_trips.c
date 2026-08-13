#include <errno.h>
#include <stdbool.h>
#include <sys/socket.h>
#include <unistd.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0001: socket create and close round trips">

test_rc_t test_0001_socket_create_and_close_round_trips(void)
{
    //* When
    // A socket is created, closed, and closed a second time. The second close
    // is the point: a table that kept the entry would report success twice.
    const int fd = socket(AF_INET, SOCK_STREAM, 0);
    const int first_close = (fd >= 0) ? close(fd) : -1;

    errno = 0;
    const int second_close = (fd >= 0) ? close(fd) : 0;
    const int second_errno = errno;

    //* Then
    // The descriptor is one the table issued rather than a standard stream, the
    // first close released it, and the second finds nothing there.
    const bool correct = fd > 2
        && first_close == 0
        && second_close != 0
        && second_errno == EBADF;

    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
