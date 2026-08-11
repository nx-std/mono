#pragma once

#include <stdbool.h>
#include <stdint.h>

/**
 * @brief Reporting in the Test Anything Protocol, version 14.
 *
 * A run that nobody watches has to be readable by something that is not a
 * person. TAP is the format that something already reads: it is line-oriented,
 * it survives being streamed, and every language has a harness that consumes
 * it. See https://testanything.org/.
 *
 * A suite reports the same document three ways, because the three readers
 * cannot reach each other. The console has it as it happens, for whoever is
 * standing there. The SD card keeps it, because the console's screen does not.
 * The host is sent it, because the host is what a run is driven from and the
 * only reader that can act on it.
 *
 * `int32_t` rather than `test_rc_t` in these signatures on purpose: the harness
 * includes this file to report through it, so this file cannot include the
 * harness back.
 */

/**
 * @brief Opens the document. Called once, before any case runs.
 *
 * What follows the version line is what a reader needs in order to know which
 * run it is looking at, since the same suite reports differently depending on
 * how the build was configured and who launched it.
 *
 * @param suite The name this suite reports under, which is also the name its
 *        file is written to and the name the runner records it as.
 * @param version The build this was compiled from.
 * @param unattended Whether the runner launched it rather than a person.
 */
void tap_begin(const char* suite, const char* version, bool unattended);

/** @brief Writes a line the protocol ignores, for whatever a reader may want. */
void tap_comment(const char* text);

/** @brief Reports one case, numbering it after the last one reported. */
void tap_case(const char* title, int32_t rc);

/**
 * @brief Reports a case the harness itself could not run.
 *
 * Distinct from a case that failed: what went wrong was the machinery around
 * the test rather than the thing under test. It is still counted and numbered,
 * because a case that did not run is not a case that passed.
 */
void tap_harness_error(const char* title, const char* reason);

/**
 * @brief Closes the document by stating how many cases there were.
 *
 * The count is only known once they have all run, which is why this is the end
 * of the document rather than the start; the protocol allows either.
 */
void tap_plan(void);

/**
 * @brief Writes the document to the SD card and sends it to the host.
 *
 * Called once the cases are over. Nothing here runs while they do: the network
 * this brings up would otherwise be sharing the process with the threads and
 * timings under test.
 *
 * The host is sent nothing unless one is listening — a suite launched from the
 * homebrew menu by hand has no host, and the file is then the whole report.
 *
 * @param suite The name the document is filed under.
 * @param network_already_up Whether the caller owns a socket driver that is
 *        already running. A suite that brought one up for its own cases has to
 *        say so, since bringing up a second would fail and taking this one down
 *        would pull it out from under the caller.
 */
void tap_report(const char* suite, bool network_already_up);
