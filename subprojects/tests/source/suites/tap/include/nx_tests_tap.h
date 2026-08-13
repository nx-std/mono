#pragma once

#include <stdbool.h>
#include <stdint.h>

// For `struct in_addr`, which is what the runtime records the host as.
#include <netinet/in.h>

#include <switch.h>

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
 * The protocol itself is the `nx-tests-tap` crate beside this file, which
 * accumulates the document and writes it to all three. This header is its C
 * surface, in two halves:
 *
 * - The `__nx_tests_tap__*` entry points, which are what the crate exports.
 * - A handful of `tap_*` wrappers over them, which gather the facts about a run
 *   that only the C runtime holds and the crate is deliberately not allowed to
 *   reach for: the system version, the host's address, and the socket driver,
 *   whose lifecycle belongs to the program because bringing one up needs the
 *   service-manager session the runtime owns.
 *
 * A suite calls the wrappers. `int32_t` rather than `test_rc_t` throughout on
 * purpose: the harness includes this file to report through it, so this file
 * cannot include the harness back.
 */

/**
 * @brief What a reader needs in order to know which run it is looking at.
 *
 * The same suite reports differently depending on how the build was configured
 * and who launched it, so none of this can be inferred from the case list.
 */
typedef struct {
    /** The name this suite reports under, and the name its file is written to. */
    const char* suite;
    /** The build this was compiled from. */
    const char* build;
    /** The directory the report is filed in. */
    const char* report_dir;
    /** The system version, as `hosversionGet` reports it, taken apart. */
    uint8_t hos_major;
    uint8_t hos_minor;
    uint8_t hos_micro;
    /** Whether the run is happening under a custom firmware. */
    bool atmosphere;
    /** Whether the runner launched this rather than a person. */
    bool unattended;
} NxTestsTapRun;

/**
 * @brief Opens the document. Called once, before any case runs.
 *
 * Anything reported before this is dropped: there is nowhere to put it, and a
 * document that silently began itself would be one whose preamble said nothing
 * about the run.
 */
void __nx_tests_tap__begin(const NxTestsTapRun* run);

/** @brief Writes a line the protocol ignores, for whatever a reader may want. */
void __nx_tests_tap__comment(const char* text);

/** @brief Reports one case, numbering it after the last one reported. */
void __nx_tests_tap__case(const char* title, int32_t rc);

/**
 * @brief Reports a case the harness itself could not run.
 *
 * Distinct from a case that failed: what went wrong was the machinery around
 * the test rather than the thing under test. It is still counted and numbered,
 * because a case that did not run is not a case that passed.
 */
void __nx_tests_tap__harness_error(const char* title, const char* reason);

/**
 * @brief Closes the document by stating how many cases there were.
 *
 * The count is only known once they have all run, which is why this is the end
 * of the document rather than the start; the protocol allows either.
 */
void __nx_tests_tap__plan(void);

/**
 * @brief Writes the document to the SD card and sends it to the host.
 *
 * @param host The address this program was pushed from, as the runtime recorded
 *        it, or zero for a suite launched by hand — which has no host, and for
 *        which the file is then the whole report. A caller that passes a
 *        non-zero address guarantees a socket driver is already running.
 *
 * @return 0 when both the card and the host took the document, -1 when either
 *         did not. Neither is a failure of the run.
 */
int32_t __nx_tests_tap__report(uint32_t host);

/**
 * @brief Opens the document, with what the runtime says about this run.
 *
 * @param suite The name this suite reports under, which is also the name its
 *        file is written to and the name the runner records it as.
 * @param version The build this was compiled from.
 * @param report_dir The directory the report is filed in. The rig's policy
 *        rather than the protocol's, so it is passed in rather than known here.
 * @param unattended Whether the runner launched it rather than a person.
 */
static inline void tap_begin(const char* suite, const char* version, const char* report_dir,
                             bool unattended)
{
    const u32 hos = hosversionGet();

    const NxTestsTapRun run = {
        .suite = suite,
        .build = version,
        .report_dir = report_dir,
        .hos_major = (uint8_t)HOSVER_MAJOR(hos),
        .hos_minor = (uint8_t)HOSVER_MINOR(hos),
        .hos_micro = (uint8_t)HOSVER_MICRO(hos),
        .atmosphere = hosversionIsAtmosphere(),
        .unattended = unattended,
    };
    __nx_tests_tap__begin(&run);
}

/** @brief Writes a line the protocol ignores, for whatever a reader may want. */
static inline void tap_comment(const char* text)
{
    __nx_tests_tap__comment(text);
}

/** @brief Reports one case, numbering it after the last one reported. */
static inline void tap_case(const char* title, int32_t rc)
{
    __nx_tests_tap__case(title, rc);
}

/** @brief Reports a case the harness itself could not run. */
static inline void tap_harness_error(const char* title, const char* reason)
{
    __nx_tests_tap__harness_error(title, reason);
}

/** @brief Closes the document by stating how many cases there were. */
static inline void tap_plan(void)
{
    __nx_tests_tap__plan();
}

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
 * @param network_already_up Whether the caller owns a socket driver that is
 *        already running. A suite that brought one up for its own cases has to
 *        say so, since bringing up a second would fail and taking this one down
 *        would pull it out from under the caller.
 */
static inline void tap_report(bool network_already_up)
{
    // The runtime records the host that pushed this program, and its absence is
    // how a suite launched by hand knows there is nobody to send anything to.
    const uint32_t host = (uint32_t)__nxlink_host.s_addr;

    bool network_is_ours = false;
    if (host != 0 && !network_already_up) {
        network_is_ours = R_SUCCEEDED(socketInitializeDefault());
    }

    // A host that cannot be reached for want of a driver is reported as no host
    // at all: the card still gets the document, which is the whole report for a
    // run nobody is listening to.
    const bool reachable = host != 0 && (network_already_up || network_is_ours);
    __nx_tests_tap__report(reachable ? host : 0);

    if (network_is_ours) {
        socketExit();
    }
}
