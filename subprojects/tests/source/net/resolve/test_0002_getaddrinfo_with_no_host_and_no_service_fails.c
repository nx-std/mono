#include <netdb.h>
#include <stdbool.h>
#include <stddef.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0002: getaddrinfo with no host and no service fails">

test_rc_t test_0002_getaddrinfo_with_no_host_and_no_service_fails(void)
{
    //* When
    // Neither a node nor a service: there is nothing to resolve, and the
    // refusal has to come from the argument check rather than from a round
    // trip that had nothing to ask about.
    struct addrinfo* list = NULL;
    const int rc = getaddrinfo(NULL, NULL, NULL, &list);

    //* Then
    // A failure code, and the caller's slot left holding nothing — a chain
    // reported alongside a failure is one nobody would think to free.
    const bool correct = rc != 0 && list == NULL;

    if (list != NULL) {
        freeaddrinfo(list);
    }
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
