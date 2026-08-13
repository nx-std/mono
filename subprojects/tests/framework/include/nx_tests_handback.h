#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

#include <switch.h>

// For the recording the tally below is counted out of.
#include "nx_tests_harness.h"

/**
 * @brief How the runner says where it can be found.
 *
 * A suite the runner launched replaces the runner on screen, and the process
 * loader runs whatever the suite asks for next: without an answer to that, it
 * falls back to the homebrew menu and the run is over after one suite. The
 * runner therefore tells every suite where it was loaded from, and the suite
 * hands control back there when it is done.
 *
 * Its presence is also how a suite knows nobody is watching it. A suite the
 * runner launched has no reason to wait for a button that will not be pressed,
 * so it runs its cases and leaves.
 *
 * The path follows the `=` with no quoting of its own; the command line the
 * loader parses quotes the argument as a whole.
 */
#define HANDBACK_ARG_PREFIX "--nx-tests-runner="

/**
 * @brief How a suite says what it found, on its way back to the runner.
 *
 * The value is `<suite>:<passed>:<failed>:<skipped>`. It travels on the command
 * line the suite hands back because that is the one channel that already exists
 * between the two, and it arrives exactly when the runner starts: the runner is
 * a fresh process every time, so a result it is not told at startup is a result
 * it never learns.
 */
#define HANDBACK_RESULT_PREFIX "--nx-tests-result="

/**
 * @brief Finds the value of an argument this program was launched with.
 *
 * Both sides of the hand-back read arguments the other one wrote, so they agree
 * on how to find them here rather than each spelling it out.
 *
 * @return The text after the prefix, or `NULL` when no argument carries it. The
 *         text is the loader's own copy and lives as long as the program does.
 */
static inline const char* handback_find_arg(const char* prefix)
{
    // The command line the runtime built. Neither global appears in a public
    // header: the runtime keeps them for the C standard library's own use, and
    // these arguments are addressed to the program rather than to it.
    extern int __system_argc;
    extern char** __system_argv;

    if (__system_argv == NULL) {
        return NULL;
    }

    const size_t prefix_len = strlen(prefix);

    // From 1: argv[0] is the path this program was loaded from, never an
    // argument addressed to it.
    for (int i = 1; i < __system_argc; i++) {
        const char* arg = __system_argv[i];
        if (arg != NULL && strncmp(arg, prefix, prefix_len) == 0) {
            return arg + prefix_len;
        }
    }

    return NULL;
}

/**
 * @brief Whether the runner launched this suite rather than a person.
 *
 * A suite in front of a person waits to be dismissed, because what it printed
 * is only readable for as long as it stays on screen. A suite the runner
 * launched has no such reader: it is one of several, its output is recorded
 * where the run can be read afterwards, and waiting for a button nobody will
 * press is what stops a run from finishing on its own.
 */
static inline bool suite_is_unattended(void)
{
    return handback_find_arg(HANDBACK_ARG_PREFIX) != NULL;
}

/** @brief How a suite's cases came out. */
typedef struct {
    int passed;
    int failed;
    int skipped;
} SuiteTally;

/** @brief Counts what the harness recorded over the course of the suite. */
static inline SuiteTally suite_tally(void)
{
    SuiteTally tally = { .passed = 0, .failed = 0, .skipped = 0 };

    const int count = g_test_result_count;
    for (int i = 0; i < count && i < TEST_RESULTS_CAPACITY; i++) {
        const test_rc_t rc = g_test_results[i].rc;
        if (rc == TEST_SUCCESS) {
            tally.passed++;
        } else if (rc == TEST_SKIPPED || rc == TEST_TODO) {
            tally.skipped++;
        } else {
            // A case whose fixture could not be built is counted here too. It
            // clears nothing, and a run containing one is not a run that passed.
            tally.failed++;
        }
    }

    return tally;
}

/**
 * @brief Reports this suite's results and hands control back to the runner.
 *
 * Call it on the way out of `main`. A suite launched from the homebrew menu
 * finds no runner to report to and leaves the way it always has.
 *
 * The process loader keeps the request until this program exits, so asking here
 * costs nothing and changes nothing about how the suite runs.
 *
 * @param suite The name this suite is recorded under. It must not contain `:`,
 *        which separates the counts that follow it.
 */
static inline void handback_to_runner(const char* suite)
{
    if (!envHasNextLoad()) {
        return;
    }

    const char* runner = handback_find_arg(HANDBACK_ARG_PREFIX);
    if (runner == NULL || runner[0] == '\0') {
        return;
    }

    const SuiteTally tally = suite_tally();

    // The runner is launched the way anything is: with its own path as argv[0],
    // quoted the way the loader parses a command line, and what happened here
    // after it.
    char cmdline[512];
    const int written =
        snprintf(cmdline, sizeof(cmdline), "\"%s\" \"" HANDBACK_RESULT_PREFIX "%s:%d:%d:%d\"",
                 runner, suite, tally.passed, tally.failed, tally.skipped);
    if (written < 0 || (size_t)written >= sizeof(cmdline)) {
        return;
    }

    envSetNextLoad(runner, cmdline);
}
