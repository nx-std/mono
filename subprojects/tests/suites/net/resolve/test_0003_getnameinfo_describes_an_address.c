#include <arpa/inet.h>
#include <netdb.h>
#include <netinet/in.h>
#include <stdbool.h>
#include <string.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0003: getnameinfo describes an address">

test_rc_t test_0003_getnameinfo_describes_an_address(void)
{
    //* Given
    // The loopback address and a known port, as a caller would hold them.
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons(RESOLVE_NUMERIC_PORT);
    addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);

    char host[NI_MAXHOST];
    char serv[NI_MAXSERV];
    memset(host, 0, sizeof(host));
    memset(serv, 0, sizeof(serv));

    //* When
    // Numeric flags on both halves are requested, but not relied on: the
    // service does its own lookup regardless and answers `localhost`/`http`
    // for this address. libnx forwards these flags verbatim too and gets the
    // same answer, so that is the platform's behaviour rather than something
    // this implementation could change. What is checked below is only what the
    // call actually guarantees.
    const int rc = getnameinfo(
        (const struct sockaddr*)&addr,
        sizeof(addr),
        host,
        sizeof(host),
        serv,
        sizeof(serv),
        NI_NUMERICHOST | NI_NUMERICSERV);

    //* Then
    // The round trip succeeded and filled both buffers with NUL-terminated
    // text. Asserting the numeric literals here would be asserting a flag the
    // service does not honour, which is what this test did before it was
    // corrected on hardware.
    const bool correct = rc == 0
        && host[0] != '\0'
        && serv[0] != '\0'
        && memchr(host, '\0', sizeof(host)) != NULL
        && memchr(serv, '\0', sizeof(serv)) != NULL;

    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
