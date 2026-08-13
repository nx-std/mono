// The test runner.
//
// A test binary can be put in front of a person by hand, from the homebrew
// menu, one at a time. A test *run* cannot: it is several binaries, and someone
// has to launch each one and read what it printed before it is replaced by the
// next. This program takes that person out of the loop. It serves the netloader
// protocol, so the host's `nxlink` (or `cargo nx link`, which speaks the same
// protocol) can push a test binary onto the console at any moment, and hands
// what it receives to the process loader to run.
//
// It stays deliberately small. Anything it needs, a suite it launches cannot
// have, because both run in the same process one after another: the network it
// brings up, it also has to take down. So it brings up nothing it does not need
// and holds nothing it does not use.

#include <inttypes.h>
#include <netinet/in.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

#include <switch.h>

#include "nx_tests_rig.h"
#include "nx_tests_handback.h"
#include "ledger.h"
#include <nx_netloader.h>

/**
 * @brief How long the runner waits between looks at the network.
 *
 * Nothing here is in a hurry: this is the interval at which a host's ping is
 * answered and a button press is noticed, and a tenth of a second is
 * imperceptible for both.
 */
#define RUNNER_POLL_INTERVAL_NS 100000000ULL

/**
 * @brief How long a line of the screen can be.
 *
 * The longest thing a line has to hold is the reason a transfer failed, dressed
 * in the escapes that colour it, so it is sized from that.
 */
#define RUNNER_LINE_SIZE (NX_NETLOADER_ERROR_SIZE + 64)

/**
 * @brief The rule drawn across the screen between sections.
 *
 * Plain ASCII on purpose: the console draws a fixed font, and a box-drawing
 * character that it has no glyph for is a hole in the line rather than a line.
 */
#define RUNNER_RULE "==============================================================================="

/** @brief The lighter rule the results table is ruled with. */
#define RUNNER_TABLE_RULE "  ------------------------------------------"

// Each column is headed in its own colour, so a count and the thing it counts
// are the same hue. The plain forms of those colours are used rather than the
// bold ones the `CONSOLE_*` names give: a heading that is as loud as the number
// under it competes with it.
#define RUNNER_HEAD_SUITE CONSOLE_ESC(37m)
#define RUNNER_HEAD_OK CONSOLE_ESC(32m)
#define RUNNER_HEAD_FAILED CONSOLE_ESC(31m)
#define RUNNER_HEAD_SKIPPED CONSOLE_ESC(33m)

/** @brief What the screen currently shows. */
typedef struct {
    /** The console's own address, in the layout `gethostid` returns. */
    uint32_t address;
    /** What the runner is doing. */
    char status[RUNNER_LINE_SIZE];
    /** The line under it: transfer progress, or why something did not work. */
    char detail[RUNNER_LINE_SIZE];
} RunnerScreen;

/**
 * @brief The screen.
 *
 * There is one console and one thing being shown on it, so the state of the
 * screen is not something a caller could hold a second copy of.
 */
static RunnerScreen g_screen;

/**
 * @brief The run in progress.
 *
 * One run is in progress at a time, for the same reason there is one screen,
 * and every suite that has reported so far is in it.
 */
static Ledger g_ledger;

/** @brief Draws the title and what it is running on. */
static void screen_draw_header(void)
{
    printf(CONSOLE_CYAN "NX-TESTS RUNNER" CONSOLE_RESET "  %s\n", VERSION);

    const u32 hos = hosversionGet();
    printf("HOS %d.%d.%d%s\n", HOSVER_MAJOR(hos), HOSVER_MINOR(hos), HOSVER_MICRO(hos),
           hosversionIsAtmosphere() ? " (AMS)" : "");

    printf(RUNNER_RULE "\n");
}

/** @brief Draws where a host should push to, and where what it pushes lands. */
static void screen_draw_network(void)
{
    if (g_screen.address == INADDR_LOOPBACK) {
        printf("  Listening   " CONSOLE_YELLOW "no network" CONSOLE_RESET "\n");
    } else {
        printf("  Listening   %" PRIu32 ".%" PRIu32 ".%" PRIu32 ".%" PRIu32 ":%d\n",
               g_screen.address & 0xFF, (g_screen.address >> 8) & 0xFF,
               (g_screen.address >> 16) & 0xFF, (g_screen.address >> 24) & 0xFF,
               NX_NETLOADER_SERVER_PORT);
    }

    printf("  Programs    %s\n", RIG_DIR);
}

/**
 * @brief Draws one row of counts, colouring only what is worth looking at.
 *
 * The columns are padded by the conversions themselves rather than by the
 * colouring, which the console consumes without spending a column on it, so the
 * table stays aligned whichever counts happen to be coloured.
 */
static void screen_draw_counts(int passed, int failed, int skipped)
{
    printf(passed > 0 ? CONSOLE_GREEN "%8d" CONSOLE_RESET : "%8d", passed);
    printf(failed > 0 ? CONSOLE_RED "%8d" CONSOLE_RESET : "%8d", failed);
    printf(skipped > 0 ? CONSOLE_YELLOW "%9d" CONSOLE_RESET : "%9d", skipped);
    printf("\n");
}

/**
 * @brief Draws what the run has produced so far.
 *
 * Nothing is drawn before the first suite reports, so a run that has not
 * started yet does not show an empty table where its results will go.
 */
static void screen_draw_results(void)
{
    if (g_ledger.count == 0 && g_ledger.dropped == 0) {
        return;
    }

    // The colouring sits outside each conversion, so the width is spent on the
    // heading alone and the columns line up with the counts below them.
    printf("\n  " RUNNER_HEAD_SUITE "%-16s" CONSOLE_RESET, "SUITE");
    printf(RUNNER_HEAD_OK "%8s" CONSOLE_RESET, "OK");
    printf(RUNNER_HEAD_FAILED "%8s" CONSOLE_RESET, "FAILED");
    printf(RUNNER_HEAD_SKIPPED "%9s" CONSOLE_RESET "\n", "SKIPPED");
    printf(RUNNER_TABLE_RULE "\n");

    for (int i = 0; i < g_ledger.count; i++) {
        const LedgerEntry* entry = &g_ledger.entries[i];
        printf("  %-16s", entry->suite);
        screen_draw_counts(entry->passed, entry->failed, entry->skipped);
    }

    printf(RUNNER_TABLE_RULE "\n");

    const LedgerTotals totals = ledger_totals(&g_ledger);
    printf("  %-16s", "TOTAL");
    screen_draw_counts(totals.passed, totals.failed, totals.skipped);

    if (g_ledger.dropped > 0) {
        printf("  " CONSOLE_YELLOW "and %d more, which this table has no room for" CONSOLE_RESET
               "\n",
               g_ledger.dropped);
    }

    // The one line worth reading from across the room. Suites the table had no
    // room for are not in the totals, so a run that overflowed cannot be called
    // passed on the strength of what fitted.
    printf("\n  ");
    if (totals.failed > 0) {
        printf(CONSOLE_RED "%d FAILED" CONSOLE_RESET "\n", totals.failed);
    } else if (g_ledger.dropped > 0) {
        printf(CONSOLE_YELLOW "INCOMPLETE" CONSOLE_RESET "\n");
    } else {
        printf(CONSOLE_GREEN "PASSED" CONSOLE_RESET "\n");
    }
}

/** @brief Draws what the runner is doing, and what the buttons do. */
static void screen_draw_status(void)
{
    printf("\n" RUNNER_RULE "\n");

    printf("%s\n", g_screen.status);
    if (g_screen.detail[0] != '\0') {
        printf("%s\n", g_screen.detail);
    }

    printf("\n  " CONSOLE_CYAN "+" CONSOLE_RESET "  exit      " CONSOLE_CYAN "-" CONSOLE_RESET
           "  clear results\n");
}

/** @brief Draws the whole screen and puts it up. */
static void screen_draw(void)
{
    consoleClear();

    screen_draw_header();
    printf("\n");
    screen_draw_network();
    screen_draw_results();
    screen_draw_status();

    consoleUpdate(NULL);
}

/** @brief Replaces what the screen says and redraws it. */
static void screen_show(const char* status, const char* detail)
{
    snprintf(g_screen.status, sizeof(g_screen.status), "%s", status);
    snprintf(g_screen.detail, sizeof(g_screen.detail), "%s", detail != NULL ? detail : "");
    screen_draw();
}

/**
 * @brief What the progress callback remembers between chunks.
 *
 * A transfer arrives in chunks far smaller than a percent of the file, and
 * redrawing the screen for each one costs more than the transfer does.
 */
typedef struct {
    /** The percentage the screen was last drawn with, or -1 when it shows none. */
    int percent;
} RunnerProgress;

static void on_progress(const char* name, size_t received, size_t total, void* ctx)
{
    RunnerProgress* progress = ctx;

    const int percent = total > 0 ? (int)((received * 100) / total) : 100;
    if (percent == progress->percent) {
        return;
    }
    progress->percent = percent;

    snprintf(g_screen.detail, sizeof(g_screen.detail), "  %s  %d%%  (%zu / %zu KiB)", name, percent,
             received / 1024, total / 1024);
    screen_draw();
}

/**
 * @brief Ends the run on screen and on the card.
 *
 * The results of a finished run stay up until they are cleared, so that a run
 * that ended while nobody was watching can still be read. This is how the next
 * one gets a clean table without restarting the runner.
 */
static void clear_results(void)
{
    ledger_clear(&g_ledger);
    screen_show("Waiting for a program", "  Results cleared.");
}

/** @brief Waits for the user to leave, for when the runner cannot go on. */
static void wait_for_exit(PadState* pad)
{
    while (appletMainLoop()) {
        padUpdate(pad);

        const uint32_t pressed = padGetButtonsDown(pad);
        if (pressed & HidNpadButton_Plus) {
            break;
        }
        if (pressed & HidNpadButton_Minus) {
            clear_results();
        }

        svcSleepThread(RUNNER_POLL_INTERVAL_NS);
    }
}

/** @brief Describes a result the way the console's own error screens do. */
static void format_result(Result rc, char* out, size_t out_size)
{
    snprintf(out, out_size, "  2%03d-%04d", R_MODULE(rc), R_DESCRIPTION(rc));
}

/**
 * @brief Builds the argument that tells a suite where to hand control back to.
 *
 * The runner's own path is the one its loader gave it, which is the path that
 * loader will accept back. A run started with no command line has none to pass
 * on, and the suites it launches end at the homebrew menu the way they always
 * did.
 *
 * @return `false` when there is no path to pass on.
 */
static bool format_handback_arg(char* out, size_t out_size)
{
    // The command line the runtime built, which is where the loader put the
    // path it loaded this program from.
    extern int __system_argc;
    extern char** __system_argv;

    if (__system_argc < 1 || __system_argv == NULL || __system_argv[0] == NULL) {
        return false;
    }

    const int written = snprintf(out, out_size, HANDBACK_ARG_PREFIX "%s", __system_argv[0]);
    return written > 0 && (size_t)written < out_size;
}

int main(void)
{
    consoleInit(NULL);

    // Configure our supported input layout: a single player with standard controller styles
    padConfigureInput(1, HidNpadStyleSet_NpadStandard);

    // Initialize the default gamepad (which reads handheld mode inputs as well as the first
    // connected controller)
    PadState pad;
    padInitializeDefault(&pad);

    // Handing a program over to be run next is the whole point of this one, and
    // an environment that cannot do it will not tell us any later than it does
    // now, so ask before bringing anything else up.
    if (!envHasNextLoad()) {
        screen_show(CONSOLE_RED "this environment cannot launch another program" CONSOLE_RESET,
                    "  Launch the runner from the homebrew menu.");
        wait_for_exit(&pad);
        consoleExit(NULL);
        return 1;
    }

    // A suite reports on its way back here, so what it found arrives as an
    // argument to this program. Being handed one is also what separates a run
    // already under way from a run starting now, and only a suite ever hands
    // one over: started any other way — pushed over the network, launched from
    // the homebrew menu — this is a new runner for a new run, and whatever an
    // earlier one left behind is over.
    ledger_load(&g_ledger);
    const char* report = handback_find_arg(HANDBACK_RESULT_PREFIX);
    if (report != NULL) {
        // A report that cannot be read costs the run one entry. The run itself
        // is unaffected, and there is nothing better to do with it here.
        (void)ledger_record(&g_ledger, report);
    } else {
        ledger_clear(&g_ledger);
    }

    char detail[RUNNER_LINE_SIZE];

    const Result socket_rc = socketInitializeDefault();
    if (R_FAILED(socket_rc)) {
        format_result(socket_rc, detail, sizeof(detail));
        screen_show(CONSOLE_RED "cannot bring up the network" CONSOLE_RESET, detail);
        wait_for_exit(&pad);
        consoleExit(NULL);
        return 1;
    }

    g_screen.address = (uint32_t)gethostid();

    NxNetloaderServer* server = __nx_netloader__server_open(RIG_DIR);
    if (server == NULL) {
        screen_show(CONSOLE_RED "cannot listen for programs" CONSOLE_RESET,
                    "  Another program may already hold the port.");
        wait_for_exit(&pad);
        socketExit();
        consoleExit(NULL);
        return 1;
    }

    // The result of the program launched last time, which is how a run that
    // ended badly reports itself: by then this program was not running.
    const Result last_load_rc = envGetLastLoadResult();
    if (R_FAILED(last_load_rc)) {
        format_result(last_load_rc, detail, sizeof(detail));
        screen_show("Waiting for a program" CONSOLE_RED "  (the last one failed to load)"
                    CONSOLE_RESET,
                    detail);
    } else {
        screen_show("Waiting for a program", NULL);
    }

    // Every suite is told where to come back to, so that a run does not end
    // with the first one.
    char handback[NX_NETLOADER_PATH_SIZE];
    const bool has_handback = format_handback_arg(handback, sizeof(handback));

    RunnerProgress progress = { .percent = -1 };
    bool launching = false;
    // Set the moment either socket stops working, and cleared once they have been rebuilt.
    bool server_lost = false;

    // Waiting for a host is exactly what the console's idle timer counts as nobody being
    // there, and sleeping takes the network down with it: the sockets stop answering while
    // this program keeps running, so a run breaks off with the host reporting it found no
    // console rather than anything about the tests.
    //
    // Nothing here can undo that once it has happened, because the code that would ask to
    // be woken is not running while the console sleeps. Not starting is the only remedy.
    // Asked for rather than insisted on: a console that refuses simply keeps its timer, and
    // a run watched by a person works either way. Held only for as long as this program
    // waits, and given back below.
    appletSetAutoSleepDisabled(true);

    while (appletMainLoop()) {
        padUpdate(&pad);

        const uint32_t pressed = padGetButtonsDown(&pad);
        if (pressed & HidNpadButton_Plus) {
            break;
        }
        if (pressed & HidNpadButton_Minus) {
            clear_results();
        }

        // A console that has slept took its network down and brought a new one up, leaving
        // both sockets bound to something that no longer exists. Neither half of the
        // protocol can report that on its own — the discovery socket simply stops hearing
        // pings, and the listening socket stops being connected to — so a failure on
        // either is what says the pair has to be rebuilt.
        if (__nx_netloader__server_answer_discovery(server) != 0) {
            server_lost = true;
        }

        if (server_lost) {
            // Freed and opened rather than rebuilt in one call: a rebuild that consumed the
            // server would leave nothing to retry with once it failed, and failing is the ordinary
            // case here for as long as the network is still down.
            __nx_netloader__server_free(server);
            server = __nx_netloader__server_open(RIG_DIR);
            if (server != NULL) {
                server_lost = false;
                screen_show("Waiting for a program", "  (listening again)");
            } else {
                // Binding fails for as long as the network is still down, so this is
                // retried on the next pass rather than given up on.
                screen_show(CONSOLE_RED "the network went away" CONSOLE_RESET,
                            "  Waiting for it to come back.");
            }
            svcSleepThread(RUNNER_POLL_INTERVAL_NS);
            continue;
        }

        NxNetloaderOutcome outcome;
        switch (__nx_netloader__server_receive(server, &outcome, has_handback ? handback : NULL,
                                               on_progress, &progress)) {
        case NX_NETLOADER_IDLE:
            break;

        case NX_NETLOADER_SERVER_LOST:
            snprintf(detail, sizeof(detail), CONSOLE_RED "  %s" CONSOLE_RESET, outcome.error);
            screen_show("Waiting for a program", detail);
            server_lost = true;
            break;

        case NX_NETLOADER_RECEIVED: {
            const Result rc = envSetNextLoad(outcome.path, outcome.cmdline);
            if (R_SUCCEEDED(rc)) {
                screen_show("Launching", outcome.path);
                launching = true;
            } else {
                format_result(rc, detail, sizeof(detail));
                screen_show(CONSOLE_RED "cannot launch what arrived" CONSOLE_RESET, detail);
            }
            break;
        }

        case NX_NETLOADER_FAILED:
            snprintf(detail, sizeof(detail), CONSOLE_RED "  %s" CONSOLE_RESET, outcome.error);
            screen_show("Waiting for a program", detail);
            progress.percent = -1;
            break;
        }

        if (launching) {
            break;
        }

        svcSleepThread(RUNNER_POLL_INTERVAL_NS);
    }

    // Everything this program holds has to be let go before the next one starts:
    // it runs in this same process, and a session left open here is one it
    // cannot open for itself. The idle timer is one of those things: a console
    // left unable to sleep by a runner that has finished would stay awake until
    // somebody noticed.
    appletSetAutoSleepDisabled(false);
    __nx_netloader__server_free(server);
    socketExit();
    consoleExit(NULL);
    return 0;
}
