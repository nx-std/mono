#pragma once

#include "nx_tests_harness.h"

/**
 * @brief Test that the process starts in the directory the program was loaded from.
 *
 * The runtime derives that directory from `argv[0]` before `main` runs, so this
 * asks for the working directory and checks it against the path the loader
 * launched. A runtime that never changed directory reports the device's root
 * instead.
 */
test_rc_t test_0001_startup_cwd_is_the_program_directory(void);

/**
 * @brief Test that a path carrying neither device nor directory resolves beside the program.
 *
 * Looks the program's own file up by its bare name, which only resolves if the
 * startup change of directory made the program's device the default one as well
 * as setting the directory on it.
 */
test_rc_t test_0002_a_bare_path_resolves_next_to_the_program(void);

/**
 * Test suite for the working directory the runtime starts in.
 */
static void rt_cwd_suite(void) {
    TEST_SUITE("rt/cwd");

    TEST_CASE(
        "Test 0001: startup_cwd_is_the_program_directory",
        test_0001_startup_cwd_is_the_program_directory
    )
    TEST_CASE(
        "Test 0002: a_bare_path_resolves_next_to_the_program",
        test_0002_a_bare_path_resolves_next_to_the_program
    )
}
