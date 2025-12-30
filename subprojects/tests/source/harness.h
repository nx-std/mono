#pragma once

#include <stdint.h>
#include <stdio.h>

/**
 * @brief The result code for a test case.
 */
typedef int32_t test_rc_t;

/**
 * Test suite function
 */
typedef void (*TestSuiteFn)(void);

/**
 * @brief The success result code for a test case.
 */
#define TEST_SUCCESS ((test_rc_t)0)

/**
 * @brief The assertion failure code for a test case.
 */
#define TEST_ASSERTION_FAILED ((test_rc_t)-101)

/**
 * @brief The failure code for a test case not implemented.
 */
#define TEST_TODO ((test_rc_t)-501)


/**
 * Test suite declaration.
 *
 * @param suite_name The name of the test suite.
 */
#define TEST_SUITE(suite_name) \
    printf("\n" CONSOLE_CYAN "TEST SUITE:" CONSOLE_RESET " " suite_name "\n\n");

/**
 * Test case declaration.
 *
 * @param test_title The title of the test case.
 * @param test_func The function to run for the test case.
 */
#define TEST_CASE(test_title, test_func) \
    { \
        printf(test_title ": "); \
        test_rc_t test_res = test_func(); \
        if (test_res == TEST_SUCCESS) { \
            printf(CONSOLE_GREEN "OK" CONSOLE_RESET "\n"); \
        } else if (test_res == TEST_TODO) { \
            printf(CONSOLE_MAGENTA "TODO" CONSOLE_RESET "\n"); \
        } else { \
            printf(CONSOLE_RED "FAILED" CONSOLE_RESET " (%d)\n", test_res); \
        } \
    }

/**
 * Skipped test case declaration.
 *
 * @param test_title The title of the test case.
 * @param test_func The function to run for the test case. This will not be run.
 */
#define XTEST_CASE(test_title, test_func) \
    { \
        printf(test_title ": " CONSOLE_YELLOW "SKIPPED" CONSOLE_RESET "\n"); \
    }
