#include <string.h>

#include <switch.h>

#include "../harness.h"

/**
 * @brief Test that randomGet fills buffers with random data.
 * 
 * This test verifies that the random number generator:
 * 1. Fills buffers of different sizes with random data
 * 2. Does not fill buffers with all zeros
 * 3. Produces different random data for different calls
 * 
 * @return TEST_SUCCESS if the test passes, TEST_ASSERTION_FAILED otherwise.
 */
test_rc_t test_0001_rand_get_fills_buffers_with_random_data(void) {
    Result rc = 0;

    //* Given
    // Initialize buffers of different sizes
    uint8_t small_buf[16] = {0};
    uint8_t medium_buf[256] = {0};
    uint8_t large_buf[1024] = {0};
    uint8_t all_zeros[1024] = {0};

    //* When
    // Fill buffers with random data
    randomGet(small_buf, sizeof(small_buf));
    randomGet(medium_buf, sizeof(medium_buf));
    randomGet(large_buf, sizeof(large_buf));

    //* Then
    // Verify buffers are not all zeros
    if (memcmp(small_buf, all_zeros, sizeof(small_buf)) == 0 ||
        memcmp(medium_buf, all_zeros, sizeof(medium_buf)) == 0 ||
        memcmp(large_buf, all_zeros, sizeof(large_buf)) == 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }
    
    // Verify buffers are different from each other
    if (memcmp(small_buf, medium_buf, sizeof(small_buf)) == 0 ||
        memcmp(medium_buf, large_buf, sizeof(medium_buf)) == 0) {
        rc = TEST_ASSERTION_FAILED;
        goto test_cleanup;
    }

test_cleanup:
    return rc;
}
