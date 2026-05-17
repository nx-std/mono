#include <stdint.h>
#include <threads.h>

#include <switch.h>

#include "../harness.h"

//<editor-fold desc="Test 0001: detach running thread self-reclaims">

#define ITERATIONS 64
#define WORKER_SLEEP_MS 10

/**
 * @brief Sleeps the current thread for the given number of milliseconds.
 */
static inline void threadSleepMs(int64_t ms) {
    svcSleepThread(ms * 1000000);
}

/**
 * Worker that briefly sleeps — so it is still running when detached — then
 * exits, triggering the detached self-reclaim path.
 */
static int sleeper_thread(void* arg) {
    (void)arg;
    threadSleepMs(WORKER_SLEEP_MS);
    return 0;
}

test_rc_t test_0001_detach_running_thread_self_reclaims(void)
{
    //* When
    // Create each worker and detach it immediately, while it is still asleep,
    // so the detach wins the detach-vs-exit race. Each detached worker then
    // self-reclaims through the Horizon `__unmapself` port when it exits.
    for (int i = 0; i < ITERATIONS; i++) {
        thrd_t worker;
        if (thrd_create(&worker, sleeper_thread, NULL) != thrd_success) {
            return TEST_ASSERTION_FAILED;
        }
        if (thrd_detach(worker) != thrd_success) {
            return TEST_ASSERTION_FAILED;
        }
    }

    //* Then
    // Give every detached worker time to wake, exit, and self-reclaim. A broken
    // self-reclaim (bad stack switch or CAS) faults the process before this
    // returns.
    threadSleepMs(WORKER_SLEEP_MS * 8);
    return TEST_SUCCESS;
}

//</editor-fold>
