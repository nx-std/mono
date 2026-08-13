// Socket (`bsd:u`/`bsd:s` + the socket driver) end-to-end tests.
//
// Separate from `nx-tests` because these bring up a network stack. The socket
// driver claims transfer memory sized by its configuration and holds several
// IPC sessions for as long as it is up, and a console where the service refuses
// to start fails the whole suite for reasons that have nothing to do with the
// code. Keeping them apart means that cannot take the unattended suite down.
//
// The socket tests run over loopback, with both ends of each connection in this
// process. That is deliberate: it makes them self-contained, so a run does not
// depend on the console being on a network, on a peer being reachable, or on
// which address the console was assigned. What it does not cover is anything
// only a real peer would exercise — a route, an MTU, a connection that fails.
//
// The resolver tests follow the same rule with one exception. A literal address
// is still a full round trip through the resolver service, so resolving one
// covers the whole path without leaving the console; only the single test that
// resolves a public name needs the console online, and it reports a skip rather
// than a failure when it cannot, since from inside the process an offline
// console and a broken lookup look the same.
//
// What is exercised is the whole stack rather than the service's commands
// alone: newlib and the descriptor table sit above the driver, and `read` and
// `write` reach a socket only because a socket device is registered. The same
// binary links whichever way the build is configured — with `use_nx_sys_net`
// and `use_nx_net` off the calls resolve to libnx's own socket driver and
// resolver and show what the stock implementations do; with them on they
// resolve to `__nx_sys_net__*`, `__nx_rt_core__libnx_socketInitialize` and the
// `__nx_rt_core__libnx_get*` resolver entry points, and Rust owns the driver,
// the device, the sessions and the lookups. Running both is the comparison
// worth making.

#include <inttypes.h>
#include <stdio.h>

#include <switch.h>

#include "nx_tests_handback.h"
#include "nx_tests_harness.h"
#include "resolve/suite.h"
#include "socket/suite.h"

/** The one definition of the result table this binary records into. */
TEST_RESULTS_STORAGE

/**
 * Test suites
 */
static TestSuiteFn test_suites[] = {
    // net
    net_socket_suite,
    net_resolve_suite,
};

int main()
{
    consoleInit(NULL);

    // Configure our supported input layout: a single player with standard controller styles
    padConfigureInput(1, HidNpadStyleSet_NpadStandard);

    // Initialize the default gamepad (which reads handheld mode inputs as well as the first connected controller)
    PadState pad;
    padInitializeDefault(&pad);

    // Launched by the runner, this suite has no reader waiting on it and no
    // reason to wait for one back.
    const bool unattended = suite_is_unattended();

    tap_begin("net", unattended);

    // The driver is brought up once for the whole binary rather than per test.
    // It is process-wide state, so a test that initialized it would be setting
    // up for every test after it, and one that tore it down would be pulling
    // the ground out from under them.
    const Result socket_rc = socketInitialize(NULL);
    if (R_FAILED(socket_rc)) {
        char reason[64];
        snprintf(reason, sizeof(reason), "the socket driver did not come up (0x%X)", socket_rc);
        tap_comment(reason);
    }

    const uint64_t test_suites_count = sizeof(test_suites) / sizeof(TestSuiteFn);
    uint64_t curr_test_suite = 0;

    // Main loop:
    // - Display the test results
    // - Wait for the user to press + to exit
    while(appletMainLoop())
    {
        // Check if the user has pressed the + button to exit
        padUpdate(&pad);
        const uint32_t key_down = padGetButtonsDown(&pad);
        if (key_down & HidNpadButton_Plus) {
            break;
        }

        // Run the next test suite. Skipped outright when the driver never came
        // up: every case would report the same failure, which says nothing
        // about any of them.
        if (curr_test_suite < test_suites_count) {
            if (R_SUCCEEDED(socket_rc)) {
                test_suites[curr_test_suite]();
            }
            curr_test_suite++;
        } else if (unattended) {
            // Everything has run and there is nobody to read it: the runner is
            // waiting for its turn back.
            break;
        }

        consoleUpdate(NULL);
    }

    tap_plan();

    // Reported before the driver goes down, and told that it is up: reaching
    // the host needs a socket, and this suite owns the one there is.
    tap_report(R_SUCCEEDED(socket_rc));

    if (R_SUCCEEDED(socket_rc)) {
        socketExit();
    }

    // Back to the runner that launched this suite, if one did: a run is
    // several suites, and it ends here otherwise.
    handback_to_runner("net");

    consoleExit(NULL);
    return 0;
}
