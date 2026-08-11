#include "tap.h"

#include <errno.h>
#include <inttypes.h>
#include <netinet/in.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include <switch.h>

#include "harness.h"

// For the directory the test rig keeps its files in: a suite's report belongs
// beside the programs the runner received and the run it is recording.
#include "../runner/netloader.h"

/** @brief The version of the protocol this reports in. */
#define TAP_VERSION_LINE "TAP version 14\n"

/**
 * @brief How much room one case's report gets.
 *
 * The longest form is a failure, which carries an indented block naming the
 * code it failed with under a title of whatever length the case was given.
 */
#define TAP_LINE_SIZE 320

/**
 * @brief The number given to the last case reported to the console.
 *
 * The protocol numbers cases from one and expects no gaps, so this counts every
 * case rather than every case the harness found room to record: the two differ
 * only once a run has overflowed the recording table, and a report that
 * renumbered around the overflow would hide it.
 */
static int g_number;

/**
 * @brief What the run was, kept for the readers that are written to at the end.
 *
 * The console is told this as the document opens. The file and the host are
 * written once the cases are over, and they are the copies that get read
 * somewhere else, later, by someone deciding whether this run is the one they
 * are looking at, so they cannot be the copies that say less.
 */
static const char* g_version = "";
static bool g_unattended;

/**
 * @brief Writes one case's report into `out`, newline included.
 *
 * The one place that knows what the protocol looks like. Both readers go
 * through it: the console as each case finishes, and the file and the host once
 * they are all over.
 */
static void tap_format_case(char* out, size_t out_size, int number, const char* title, int32_t rc)
{
    if (rc == TEST_SUCCESS) {
        snprintf(out, out_size, "ok %d - %s\n", number, title);
        return;
    }

    // A skip is a pass that says it did not run. A todo is a failure the
    // protocol expects, so a harness reading this does not count it against
    // the run.
    if (rc == TEST_SKIPPED) {
        snprintf(out, out_size, "ok %d - %s # SKIP\n", number, title);
        return;
    }
    if (rc == TEST_TODO) {
        snprintf(out, out_size, "not ok %d - %s # TODO not implemented yet\n", number, title);
        return;
    }

    // Everything else failed, and the block under it is where the protocol puts
    // what a reader needs in order to act on the failure.
    if (rc == TEST_SETUP_FAILED) {
        snprintf(out, out_size,
                 "not ok %d - %s\n"
                 "  ---\n"
                 "  reason: the fixture could not be built, so the case never ran\n"
                 "  ...\n",
                 number, title);
        return;
    }

    snprintf(out, out_size,
             "not ok %d - %s\n"
             "  ---\n"
             "  rc: 0x%08" PRIX32 "\n"
             "  ...\n",
             number, title, (uint32_t)rc);
}

/** @brief Writes the comments that say which run this is, newline included. */
static void tap_format_preamble(char* out, size_t out_size, const char* suite)
{
    const u32 hos = hosversionGet();

    snprintf(out, out_size,
             "# suite: %s\n"
             "# build: %s\n"
             "# hos: %d.%d.%d%s\n"
             "# mode: %s\n",
             suite, g_version, HOSVER_MAJOR(hos), HOSVER_MINOR(hos), HOSVER_MICRO(hos),
             hosversionIsAtmosphere() ? " (AMS)" : "",
             g_unattended ? "unattended" : "interactive");
}

void tap_begin(const char* suite, const char* version, bool unattended)
{
    g_version = version;
    g_unattended = unattended;

    fputs(TAP_VERSION_LINE, stdout);

    char preamble[TAP_LINE_SIZE];
    tap_format_preamble(preamble, sizeof(preamble), suite);
    fputs(preamble, stdout);
}

void tap_comment(const char* text)
{
    printf("# %s\n", text);
}

void tap_case(const char* title, int32_t rc)
{
    g_number++;

    char line[TAP_LINE_SIZE];
    tap_format_case(line, sizeof(line), g_number, title, rc);
    fputs(line, stdout);
}

void tap_harness_error(const char* title, const char* reason)
{
    g_number++;

    printf("not ok %d - %s\n"
           "  ---\n"
           "  harness: %s\n"
           "  ...\n",
           g_number, title, reason);
}

void tap_plan(void)
{
    printf("1..%d\n", g_number);
}

/** @brief Writes text to whichever of the two sinks are open. */
static void tap_put(FILE* file, int host_fd, const char* text)
{
    const size_t len = strlen(text);

    if (file != NULL) {
        fwrite(text, 1, len, file);
    }

    if (host_fd >= 0) {
        // A socket takes what it has room for and says how much that was, so a
        // line goes out in as many writes as it takes.
        size_t sent = 0;
        while (sent < len) {
            const ssize_t written = write(host_fd, text + sent, len - sent);
            if (written <= 0) {
                return;
            }
            sent += (size_t)written;
        }
    }
}

/** @brief Writes the whole document out of what the harness recorded. */
static void tap_write_document(FILE* file, int host_fd, const char* suite)
{
    char line[TAP_LINE_SIZE];

    tap_put(file, host_fd, TAP_VERSION_LINE);

    tap_format_preamble(line, sizeof(line), suite);
    tap_put(file, host_fd, line);

    const int count = g_test_result_count;
    for (int i = 0; i < count && i < TEST_RESULTS_CAPACITY; i++) {
        // The table is `volatile` so that a debugger can read it out of a
        // finished process; by the time this runs the cases are over and
        // nothing is still writing, so reading it as ordinary memory is safe.
        tap_format_case(line, sizeof(line), i + 1, (const char*)g_test_results[i].title,
                        g_test_results[i].rc);
        tap_put(file, host_fd, line);
    }

    // The console saw every case; the recording table only holds so many. Where
    // they disagree the run overflowed, and a reader is told rather than handed
    // a document that quietly stops short.
    if (g_number > count) {
        snprintf(line, sizeof(line), "# %d further cases ran but the recording table was full\n",
                 g_number - count);
        tap_put(file, host_fd, line);
    }

    snprintf(line, sizeof(line), "1..%d\n", count);
    tap_put(file, host_fd, line);
}

/**
 * @brief Opens the file this run is filed under, creating the directory.
 *
 * @return `NULL` when it could not be opened, which loses the file and nothing
 *         else: the console has already shown the run and the host is sent it
 *         regardless.
 */
static FILE* tap_open_file(const char* suite)
{
    if (mkdir(NETLOADER_DROP_DIR, 0777) != 0 && errno != EEXIST) {
        return NULL;
    }

    char path[256];
    const int written = snprintf(path, sizeof(path), "%s/%s.tap", NETLOADER_DROP_DIR, suite);
    if (written < 0 || (size_t)written >= sizeof(path)) {
        return NULL;
    }

    return fopen(path, "w");
}

void tap_report(const char* suite, bool network_already_up)
{
    FILE* file = tap_open_file(suite);

    // The runtime records the host that pushed this program, and its absence is
    // how a suite launched by hand knows there is nobody to send anything to.
    int host_fd = -1;
    bool network_is_ours = false;
    if (__nxlink_host.s_addr != 0) {
        if (network_already_up || R_SUCCEEDED(socketInitializeDefault())) {
            network_is_ours = !network_already_up;
            host_fd = nxlinkConnectToHost(false, false);
        }
    }

    tap_write_document(file, host_fd, suite);

    if (file != NULL) {
        fclose(file);
    }
    if (host_fd >= 0) {
        close(host_fd);
    }
    if (network_is_ours) {
        socketExit();
    }
}
