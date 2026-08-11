#include <switch.h>

#include "../harness.h"

/**
 * @brief Test that randomGet64 returns different values on consecutive calls.
 * 
 * This test verifies that the random number generator produces different values
 * on consecutive calls, which is a basic requirement for any random number generator.
 * 
 * @return TEST_SUCCESS if the test passes, TEST_ASSERTION_FAILED otherwise.
 */
test_rc_t test_0002_rand_get64_returns_different_values(void) {
    Result rc = 0;

    //* When
    // Get two random values
    uint64_t val1 = randomGet64();
    uint64_t val2 = randomGet64();

    //* Then
    // Verify the values are different
    if (val1 == val2) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

test_cleanup:
    return rc;
} 
