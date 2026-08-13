// Filesystem (`fsp-srv` + fsdev) end-to-end tests.
//
// Separate from `nx-tests` because these touch the console's own storage. The
// SD card tests build a fixture under one directory and remove it again, so a
// run leaves the card as it found it, but a card that is missing, full or
// write-protected fails the whole suite for reasons that have nothing to do
// with the code. Keeping them apart means that cannot take the unattended suite
// down with it.
//
// The savedata tests read what the console already holds rather than creating
// anything, and skip themselves when it holds no account savedata: which saves
// exist is a property of the console, not of the code under test.
//
// What is exercised is the whole stack rather than the `fsp-srv` commands
// alone: newlib's stdio calls libsysbase, which dispatches through the
// descriptor table to fsdev, which issues the commands. The same binary links
// whichever way the build is configured — with `use_nx_service_fs` and
// `use_nx_fsdev` off, the commands and the device above them resolve to libnx
// and show what the stock implementation does; with them on, they resolve to
// `__nx_rt_nro__libnx_fs_*` and `__nx_fsdev__*` and Rust owns both the session
// and the device. Running both is the comparison worth making.

#include <inttypes.h>
#include <stdio.h>

#include <switch.h>

#include "nx_tests_handback.h"
#include "nx_tests_harness.h"
#include "savedata/suite.h"
#include "sdmc/suite.h"

/** The one definition of the result table this binary records into. */
TEST_RESULTS_STORAGE

/**
 * Test suites
 */
static TestSuiteFn test_suites[] = {
    // fs
    fs_sdmc_suite,
    fs_savedata_suite,
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

    tap_begin("fs", unattended);

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
    handback_to_runner("fs");

    consoleExit(NULL);
    return 0;
}
