#pragma once

#include <stdint.h>
#include <stdio.h>
#include <threads.h>
#include <inttypes.h>

#include "nx_sync_oneshot.h"

/**
 * @brief The result code for a test case.
 */
typedef int32_t test_rc_t;

/**
 * Test case function
 */
typedef test_rc_t (*TestCaseFn)(void);

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
 * @brief The failure code for a test case that was skipped.
 */
#define TEST_SKIPPED ((test_rc_t)-502)

/**
 * Test suite declaration.
 *
 * @param suite_name The name of the test suite.
 */
#define TEST_SUITE(suite_name) \
    printf("\n" CONSOLE_CYAN "TEST SUITE:" CONSOLE_RESET " " suite_name "\n\n");

/**
 * @brief Arguments for a test case thread.
 */
typedef struct {
    NxSyncOneshotSender* sender;
    TestCaseFn func;
} TestCaseThreadArgs;

/**
 * @brief The entry point for a test case thread.
 * @param arg A pointer to the TestCaseThreadArgs struct.
 */
static int test_case_thread_func(void* arg) {
    TestCaseThreadArgs* args = (TestCaseThreadArgs*)arg;
    test_rc_t rc = args->func();
    __nx_sync_oneshot_send(args->sender, (void*)(intptr_t)rc);
    return 0;
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

/**
 * Test case declaration.
 *
 * @param test_title The title of the test case.
 * @param test_func The function to run for the test case.
 */
#define TEST_CASE(test_title, test_func) \
    { \
        printf(test_title ": "); \
        fflush(stdout); \
        \
        NxSyncOneshotSender* sender; \
        NxSyncOneshotReceiver* receiver; \
        __nx_sync_oneshot_create(&sender, &receiver); \
        \
        TestCaseThreadArgs args = { .sender = sender, .func = test_func }; \
        thrd_t thread; \
        if (thrd_create(&thread, test_case_thread_func, &args) != thrd_success) { \
            printf(CONSOLE_RED "HARNESS_ERROR: thread_create failed" CONSOLE_RESET "\n"); \
            __nx_sync_oneshot_sender_free(sender); \
            __nx_sync_oneshot_receiver_free(receiver); \
        } else { \
            void* recv_value = NULL; \
            if (__nx_sync_oneshot_recv(receiver, &recv_value) == 0) { \
                test_rc_t test_res = (test_rc_t)(intptr_t)recv_value; \
                if (test_res == TEST_SUCCESS) { \
                    printf(CONSOLE_GREEN "OK" CONSOLE_RESET "\n"); \
                } else if (test_res == TEST_TODO) { \
                    printf(CONSOLE_MAGENTA "TODO" CONSOLE_RESET "\n"); \
                } else { \
                    printf(CONSOLE_RED "FAILED" CONSOLE_RESET " (%d)\n", test_res); \
                } \
            } else { \
                printf(CONSOLE_RED "HARNESS_ERROR: recv failed" CONSOLE_RESET "\n"); \
            } \
            thrd_join(thread, NULL); \
        } \
    }
