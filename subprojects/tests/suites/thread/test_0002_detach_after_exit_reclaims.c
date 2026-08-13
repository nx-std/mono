#include <stdint.h>
#include <threads.h>

#include <switch.h>

#include "nx_tests_harness.h"

//<editor-fold desc="Test 0002: detach after exit reclaims">

#define ITERATIONS 64
#define EXIT_SETTLE_MS 10

/**
 * @brief Sleeps the current thread for the given number of milliseconds.
 */
static inline void threadSleepMs(int64_t ms) {
    svcSleepThread(ms * 1000000);
}

/**
 * Worker that exits immediately.
 */
static int noop_thread(void* arg) {
    (void)arg;
    return 0;
}

test_rc_t test_0002_detach_after_exit_reclaims(void)
{
    //* When
    // Create each worker, wait long enough that it has surely exited, then
    // detach it — exercising the detach-after-exit path, where the detaching
    // call loses the race and reclaims the already-exited worker itself.
    for (int i = 0; i < ITERATIONS; i++) {
        thrd_t worker;
        if (thrd_create(&worker, noop_thread, NULL) != thrd_success) {
            return TEST_ASSERTION_FAILED;
        }
        threadSleepMs(EXIT_SETTLE_MS);
        if (thrd_detach(worker) != thrd_success) {
            return TEST_ASSERTION_FAILED;
        }
    }

    //* Then
    return TEST_SUCCESS;
}

//</editor-fold>
