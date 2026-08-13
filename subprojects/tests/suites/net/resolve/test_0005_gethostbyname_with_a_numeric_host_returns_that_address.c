#include <arpa/inet.h>
#include <netdb.h>
#include <netinet/in.h>
#include <stdbool.h>
#include <string.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0005: gethostbyname with a numeric host returns that address">

test_rc_t test_0005_gethostbyname_with_a_numeric_host_returns_that_address(void)
{
    //* When
    // The older resolver entry point, which answers with a `hostent` rather
    // than a chain. It reaches the service over the same session, so it covers
    // a second of the five symbols that need one.
    const struct hostent* entry = gethostbyname(RESOLVE_NUMERIC_HOST);

    //* Then
    // One IPv4 address, and it is the literal that went in. The address list
    // is walked rather than indexed, since a terminator that was never written
    // would make the first entry look right and the walk run off the end.
    bool correct = entry != NULL
        && entry->h_addrtype == AF_INET
        && entry->h_length == (int)sizeof(struct in_addr)
        && entry->h_addr_list != NULL
        && entry->h_addr_list[0] != NULL;

    if (correct) {
        struct in_addr first;
        memcpy(&first, entry->h_addr_list[0], sizeof(first));
        correct = ntohl(first.s_addr) == INADDR_LOOPBACK;
    }

    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
