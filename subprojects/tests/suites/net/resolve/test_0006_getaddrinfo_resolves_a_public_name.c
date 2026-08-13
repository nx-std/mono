#include <netdb.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0006: getaddrinfo resolves a public name">

test_rc_t test_0006_getaddrinfo_resolves_a_public_name(void)
{
    //* Given
    // Hints constraining the answer to IPv4 stream records. Unlike every other
    // test here this one needs the console to be on a network with working
    // DNS, which is a property of where the console is rather than of the code
    // under test — so a failure is reported as a skip. The cost of that is
    // real: a genuinely broken lookup is indistinguishable from an offline
    // console, and this test cannot tell them apart. What it can do is confirm
    // a real lookup when the console is online, which none of the literal
    // lookups above ever exercise.
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;

    //* When
    // A name that has to leave the console to resolve.
    struct addrinfo* list = NULL;
    const int rc = getaddrinfo(RESOLVE_PUBLIC_NAME, NULL, &hints, &list);

    //* Then
    // At least one usable IPv4 record, with the address length the family
    // implies rather than whatever the response happened to carry.
    if (rc != 0 || list == NULL) {
        if (list != NULL) {
            freeaddrinfo(list);
        }
        return TEST_SKIPPED;
    }

    bool correct = false;
    for (const struct addrinfo* it = list; it != NULL; it = it->ai_next) {
        if (it->ai_family == AF_INET && it->ai_addr != NULL
            && it->ai_addrlen >= sizeof(struct sockaddr_in)) {
            correct = true;
        }
    }

    freeaddrinfo(list);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
