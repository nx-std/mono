#pragma once

#include "../harness.h"

/**
 * @brief Test that __nx_rand_get fills buffers with random data.
 * 
 * This test verifies that the random number generator:
 * 1. Fills buffers of different sizes with random data
 * 2. Does not fill buffers with all zeros
 * 3. Produces different random data for different calls
 */
test_rc_t test_0001_rand_get_fills_buffers_with_random_data(void);

/**
 * @brief Test that __nx_rand_get64 returns different values on consecutive calls.
 * 
 * This test verifies that the random number generator produces different values
 * on consecutive calls, which is a basic requirement for any random number generator.
 */
test_rc_t test_0002_rand_get64_returns_different_values(void);

/**
 * Test suite for random number generation.
 */
static void rand_suite(void) {
    TEST_SUITE("rand");
    
    TEST_CASE(
        "Test 0001: rand_get_fills_buffers_with_random_data",
        test_0001_rand_get_fills_buffers_with_random_data
    )
    TEST_CASE(
        "Test 0002: rand_get64_returns_different_values",
        test_0002_rand_get64_returns_different_values
    )
} 
