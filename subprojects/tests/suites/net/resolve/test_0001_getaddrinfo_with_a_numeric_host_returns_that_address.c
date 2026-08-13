#include <netdb.h>
#include <stdbool.h>
#include <stddef.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0001: getaddrinfo with a numeric host returns that address">

test_rc_t test_0001_getaddrinfo_with_a_numeric_host_returns_that_address(void)
{
    //* Given
    // Hints constraining the answer to one IPv4 stream record, so the chain
    // returned is small enough to state an expectation about.
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;

    //* When
    // A literal address and a literal port. The resolver still answers this
    // through the service rather than parsing it locally, so the round trip is
    // the same one a name would make — including the service-manager session
    // the resolver session is acquired over.
    struct addrinfo* list = NULL;
    const int rc = getaddrinfo(RESOLVE_NUMERIC_HOST, RESOLVE_NUMERIC_SERVICE, &hints, &list);

    //* Then
    // The chain names the address that went in, and freeing it returns the
    // whole allocation rather than only its head.
    const bool correct = rc == 0
        && list != NULL
        && resolve_list_has_loopback(list, RESOLVE_NUMERIC_PORT);

    if (list != NULL) {
        freeaddrinfo(list);
    }
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
