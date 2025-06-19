#include <stdio.h>
#include <inttypes.h>
#include <switch.h>

#include "harness.h"
#include "rand/suite.h"
#include "sync/suite.h"

/**
 * Test suites
 */
static TestSuiteFn test_suites[] = {
    // random
    rand_suite,
    // sync
    sync_mutex_suite,
    sync_remutex_suite,
    sync_condvar_suite,
    sync_barrier_suite,
    sync_rwlock_suite,
    sync_semaphore_suite,
};

int main()
{
    consoleInit(NULL);

    // Configure our supported input layout: a single player with standard controller styles
    padConfigureInput(1, HidNpadStyleSet_NpadStandard);

    // Initialize the default gamepad (which reads handheld mode inputs as well as the first connected controller)
    PadState pad;
    padInitializeDefault(&pad);

    // Print the test header
    printf("NX-TESTS (%s)\n", VERSION);
    printf("Press + to exit\n");

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
        }

        consoleUpdate(NULL);
    }

    consoleExit(NULL);
    return 0;
}
