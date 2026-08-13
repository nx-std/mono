// Read-only filesystem (romfs) end-to-end tests.
//
// Kept in its own binary because the fixture is the binary: the image these
// tests read is packed into this NRO at build time, so the suite carries what
// it needs and touches neither the SD card nor the network. What it does depend
// on is being launched from a path it can find itself by — an NRO reaches its
// own image by opening the file the loader named on the command line — which is
// why a run of this suite says as much about the runtime and the descriptor
// table as it does about the image reader.
//
// What is exercised is the whole stack rather than the mount alone: newlib's
// stdio calls libsysbase, which dispatches through the descriptor table to the
// device, which reads the image out of this file. The same binary links
// whichever way the build is configured — with `use_nx_romfs` off the `romfs*`
// surface resolves to libnx and shows what the stock implementation does; with
// it on, the source-named mounts resolve to `__nx_romfs__*` and
// `romfsMountSelf` to `__nx_rt_nro__libnx_romfs_mount_self`. Running both is
// the comparison worth making.
//
// One case does not agree across the two, and is meant not to: mounting a name
// that is already mounted is refused here and allowed by libnx, whose romfs
// never checks and whose registration then replaces the first device. Case 0012
// fails against libnx by construction rather than by regression, the way the
// thread suite's detach cases do.

#include <inttypes.h>
#include <stdio.h>

#include <switch.h>

#include "nx_tests_handback.h"
#include "nx_tests_harness.h"
#include "suite.h"

/** The one definition of the result table this binary records into. */
TEST_RESULTS_STORAGE

/**
 * Test suites
 */
static TestSuiteFn test_suites[] = {
    // romfs
    romfs_suite,
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

    tap_begin("romfs", unattended);

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

        // Run the next test suite
        if (curr_test_suite < test_suites_count) {
            test_suites[curr_test_suite]();
            curr_test_suite++;
        } else if (unattended) {
            // Everything has run and there is nobody to read it: the runner is
            // waiting for its turn back.
            break;
        }

        consoleUpdate(NULL);
    }

    tap_plan();
    tap_report(false);

    // Back to the runner that launched this suite, if one did: a run is
    // several suites, and it ends here otherwise.
    handback_to_runner("romfs");

    consoleExit(NULL);
    return 0;
}
