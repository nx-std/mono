#pragma once

#include <stdbool.h>
#include <stdio.h>

#include <switch.h>

#include "../runner/handback.h"
#include "harness.h"

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
