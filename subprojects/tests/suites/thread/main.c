// Thread tests.
//
// Split out of `nx-tests` when that binary became the test runner: what used to be
// one unattended suite is now one binary per area, each of which the runner can
// be handed on its own.
//
// The same binary links either way: with `use_nx_sys_thread` off the calls
// resolve to libnx's own thread layer, which is the baseline to compare ours
// against — except that libnx leaves `thrd_detach` unimplemented, so the detach
// cases fail there by construction rather than by regression.

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
    // thread
    thread_suite,
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

    tap_begin("thread", unattended);

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
    handback_to_runner("thread");

    consoleExit(NULL);
    return 0;
}
