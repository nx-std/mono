#pragma once

#include <stdint.h>
#include <stdio.h>
#include <threads.h>
#include <inttypes.h>

#include "nx_sync_oneshot.h"
#include "nx_tests_framework.h"

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
 *
 * A group of cases is not something the protocol has, so it is reported as a
 * comment: a reader that groups by it can, and one that does not is unaffected.
 */
#define TEST_SUITE(suite_name) tap_comment(suite_name);

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
 *
 * Declared here and defined once per binary by `TEST_RESULTS_STORAGE`. A
 * `static` definition in this header would give every translation unit that
 * includes it its own copy — and `volatile` keeps the unused ones, so a suite
 * of thirty files would leave a reader thirty tables to search for the one that
 * was filled.
 */
extern volatile TestResult g_test_results[TEST_RESULTS_CAPACITY];

/** @brief How many entries of `g_test_results` are filled. */
extern volatile int g_test_result_count;

/**
 * @brief Defines the storage the declarations above name.
 *
 * Expanded exactly once per test binary, at file scope in its `main.c`.
 */
#define TEST_RESULTS_STORAGE \
    volatile TestResult g_test_results[TEST_RESULTS_CAPACITY]; \
    volatile int g_test_result_count = 0;

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
 *
 * `inline` like the rest of this header: most files that include it declare
 * cases without running any, and a plain `static` here is an unused function in
 * every one of them.
 *
 * @param arg A pointer to the TestCaseThreadArgs struct.
 */
static inline int test_case_thread_func(void* arg) {
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
        test_record(test_title, TEST_SKIPPED); \
        tap_case(test_title, TEST_SKIPPED); \
    }

/**
 * Test case declaration.
 *
 * @param test_title The title of the test case.
 * @param test_func The function to run for the test case.
 */
#define TEST_CASE(test_title, test_func) \
    { \
        NxSyncOneshotSender* sender; \
        NxSyncOneshotReceiver* receiver; \
        __nx_std_sync__oneshot_create(&sender, &receiver); \
        \
        TestCaseThreadArgs args = { .sender = sender, .func = test_func }; \
        thrd_t thread; \
        if (thrd_create(&thread, test_case_thread_func, &args) != thrd_success) { \
            test_record(test_title, TEST_SETUP_FAILED); \
            tap_harness_error(test_title, "the thread to run the case on could not be created"); \
            __nx_std_sync__oneshot_sender_free(sender); \
            __nx_std_sync__oneshot_receiver_free(receiver); \
        } else { \
            void* recv_value = NULL; \
            if (__nx_std_sync__oneshot_recv(receiver, &recv_value) == 0) { \
                test_rc_t test_res = (test_rc_t)(intptr_t)recv_value; \
                test_record(test_title, test_res); \
                tap_case(test_title, test_res); \
            } else { \
                test_record(test_title, TEST_SETUP_FAILED); \
                tap_harness_error(test_title, "the case's result never arrived"); \
            } \
            thrd_join(thread, NULL); \
        } \
    }
