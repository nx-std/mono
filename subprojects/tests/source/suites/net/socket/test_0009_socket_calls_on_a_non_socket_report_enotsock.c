#include <errno.h>
#include <stdbool.h>
#include <sys/socket.h>
#include <unistd.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0009: socket calls on a non-socket report ENOTSOCK">

/** Far above anything the descriptor table hands out, so nothing has it open. */
#define UNOPENED_FD 4096

test_rc_t test_0009_socket_calls_on_a_non_socket_report_enotsock(void)
{
    //* When
    // Socket calls are made against standard output, which is open but backed
    // by the console, and against a number nothing ever opened.
    errno = 0;
    const int listen_on_stdout = listen(STDOUT_FILENO, 1);
    const int listen_on_stdout_errno = errno;

    errno = 0;
    const ssize_t send_on_stdout = send(STDOUT_FILENO, "x", 1, 0);
    const int send_on_stdout_errno = errno;

    errno = 0;
    const int listen_on_unopened = listen(UNOPENED_FD, 1);
    const int listen_on_unopened_errno = errno;

    //* Then
    // The two ways a descriptor can fail to name a socket stay apart:
    // `ENOTSOCK` for one backed by something else, `EBADF` for one that is not
    // open. A lookup that collapsed them would report the same for both.
    const bool correct = listen_on_stdout != 0
        && listen_on_stdout_errno == ENOTSOCK
        && send_on_stdout < 0
        && send_on_stdout_errno == ENOTSOCK
        && listen_on_unopened != 0
        && listen_on_unopened_errno == EBADF;

    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
