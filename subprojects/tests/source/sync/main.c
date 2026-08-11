// Synchronization primitive tests.
//
// Split out of `nx-tests` because these are the suites that stand on the C ABI
// of a lock: a C caller allocates a `Mutex`, `RMutex`, `RwLock`, `Barrier` or
// `Semaphore` and hands it to a Rust implementation that has to agree, to the
// byte, about what is inside it. A disagreement does not fail cleanly — it
// reads one of the caller's fields as another and writes the rest past the end
// of the object, so the damage lands somewhere else entirely, often in another
// suite. Keeping them in their own binary means that when they do misbehave,
// what they corrupt is their own address space and not the results of every
// test that ran before them.
//
// The same binary links either way: with `use_nx_sys_sync` off the primitives
// resolve to the C implementation, which is the baseline to compare ours
// against.

#include <inttypes.h>
#include <stdio.h>

#include <switch.h>

#include "../harness.h"
#include "suite.h"

/** The one definition of the result table this binary records into. */
TEST_RESULTS_STORAGE

/**
 * Test suites
 */
static TestSuiteFn test_suites[] = {
    // sync
    sync_mutex_suite,
    sync_remutex_suite,
    sync_condvar_suite,
    sync_barrier_suite,
    sync_rwlock_suite,
    sync_semaphore_suite,
    sync_oneshot_suite,
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
    printf("NX-TESTS-SYNC (%s)\n", VERSION);
    u32 ver = hosversionGet();
    printf("HOS %d.%d.%d%s\n",
        HOSVER_MAJOR(ver), HOSVER_MINOR(ver), HOSVER_MICRO(ver),
        hosversionIsAtmosphere() ? " (AMS)" : "");
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
