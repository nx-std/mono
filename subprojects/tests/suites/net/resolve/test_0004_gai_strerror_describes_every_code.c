#include <netdb.h>
#include <stdbool.h>
#include <string.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0004: gai_strerror describes every code">

test_rc_t test_0004_gai_strerror_describes_every_code(void)
{
    //* Given
    // The codes a caller actually branches on, plus one the resolver never
    // reports — the description of an unknown code still has to be a string,
    // since a caller prints it without checking.
    static const int codes[] = {0, EAI_AGAIN, EAI_FAIL, EAI_MEMORY, EAI_NONAME, EAI_FAMILY, 12345};

    //* When
    // Each code is described. Nothing here reaches the service: this is a
    // table lookup, and it has to work whether or not a session was ever
    // established.
    const char* described[sizeof(codes) / sizeof(codes[0])];
    for (size_t i = 0; i < sizeof(codes) / sizeof(codes[0]); i++) {
        described[i] = gai_strerror(codes[i]);
    }

    //* Then
    // Every code produced a non-empty string rather than a null a caller would
    // hand straight to `printf`.
    bool correct = true;
    for (size_t i = 0; i < sizeof(codes) / sizeof(codes[0]); i++) {
        if (described[i] == NULL || described[i][0] == '\0') {
            correct = false;
        }
    }

    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
