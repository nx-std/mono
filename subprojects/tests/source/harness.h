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
 * @brief The failure code for a test case whose fixture could not be built.
 *
 * Distinct from an assertion failure on purpose: it says the test never got as
 * far as exercising what it is named after, so the thing under test is neither
 * accused nor cleared.
 */
#define TEST_SETUP_FAILED ((test_rc_t)-503)

/**
 * Test suite declaration.
 *
 * @param suite_name The name of the test suite.
 */
#define TEST_SUITE(suite_name) \
    printf("\n" CONSOLE_CYAN "TEST SUITE:" CONSOLE_RESET " " suite_name "\n\n");

/**
 * @brief How many results the recording table holds.
 *
 * Larger than any suite in this workspace; a run that exceeds it stops
 * recording rather than writing past the end, and the count says it did.
 */
#define TEST_RESULTS_CAPACITY 64

/**
 * @brief One recorded test result.
 *
 * The console draws its text straight to the framebuffer and keeps no copy, so
 * a result that was only printed cannot be read back afterwards. Recording it
 * here gives a debugger something to read out of memory once the run is over,
 * with no need to catch the test as it happens.
 */
typedef struct {
    /** The title passed to `TEST_CASE`. */
    const char* title;
    /** What the case returned. */
    test_rc_t rc;
} TestResult;

/**
 * @brief Every result the run has produced, in order.
 *
 * `volatile` because nothing in the process ever reads it: the only reader is a
 * debugger attached afterwards, which the compiler cannot see. Without it the
 * stores are dead and the whole array is optimised away, leaving a debugger
 * with a symbol that is not in the binary.
 */
static volatile TestResult g_test_results[TEST_RESULTS_CAPACITY];

/** @brief How many entries of `g_test_results` are filled. */
static volatile int g_test_result_count = 0;

/**
 * @brief Records one result, ignoring anything past the table's capacity.
 */
static inline void test_record(const char* title, test_rc_t rc) {
    const int at = g_test_result_count;
    if (at < 0 || at >= TEST_RESULTS_CAPACITY) {
        return;
    }
    g_test_results[at].title = title;
    g_test_results[at].rc = rc;
    // Written last, so a reader that catches the table mid-update sees a count
    // covering only entries that are already complete.
    g_test_result_count = at + 1;
}

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
    __nx_std_sync__oneshot_send(args->sender, (void*)(intptr_t)rc);
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
        test_record(test_title, TEST_SKIPPED); \
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
        __nx_std_sync__oneshot_create(&sender, &receiver); \
        \
        TestCaseThreadArgs args = { .sender = sender, .func = test_func }; \
        thrd_t thread; \
        if (thrd_create(&thread, test_case_thread_func, &args) != thrd_success) { \
            printf(CONSOLE_RED "HARNESS_ERROR: thread_create failed" CONSOLE_RESET "\n"); \
            __nx_std_sync__oneshot_sender_free(sender); \
            __nx_std_sync__oneshot_receiver_free(receiver); \
        } else { \
            void* recv_value = NULL; \
            if (__nx_std_sync__oneshot_recv(receiver, &recv_value) == 0) { \
                test_rc_t test_res = (test_rc_t)(intptr_t)recv_value; \
                test_record(test_title, test_res); \
                if (test_res == TEST_SUCCESS) { \
                    printf(CONSOLE_GREEN "OK" CONSOLE_RESET "\n"); \
                } else if (test_res == TEST_TODO) { \
                    printf(CONSOLE_MAGENTA "TODO" CONSOLE_RESET "\n"); \
                } else if (test_res == TEST_SKIPPED) { \
                    printf(CONSOLE_YELLOW "SKIPPED" CONSOLE_RESET "\n"); \
                } else if (test_res == TEST_SETUP_FAILED) { \
                    printf(CONSOLE_YELLOW "SETUP FAILED" CONSOLE_RESET "\n"); \
                } else { \
                    printf(CONSOLE_RED "FAILED" CONSOLE_RESET " (0x%X)\n", test_res); \
                } \
            } else { \
                printf(CONSOLE_RED "HARNESS_ERROR: recv failed" CONSOLE_RESET "\n"); \
            } \
            thrd_join(thread, NULL); \
        } \
    }
