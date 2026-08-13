#include <stdint.h>
#include <threads.h>

#include <switch.h>

#include "../harness.h"

//<editor-fold desc="Test 0003: thread pointer is per-thread and preserved">

#define WORKERS 4
#define YIELD_SLEEP_MS 50

/** Offset of `ThreadVars.tls_ptr` within the 0x200-byte Horizon TLS block. */
#define THREAD_VARS_TLS_PTR_OFFSET 0x1F8

/**
 * Reads `tpidr_el0`.
 *
 * This is the register LLVM resolves a Rust `#[thread_local]` against, and
 * Horizon does not maintain it — `nx_sys_thread_tls::init_thread_vars` writes
 * it, on the thread it initializes. Reading it here is the only way to check
 * that from outside Rust: the C toolchain uses a soft thread pointer, so a C
 * `__thread` variable would exercise `__aarch64_read_tp` instead and prove
 * nothing about this register.
 */
static inline uint64_t read_tpidr_el0(void) {
    uint64_t value;
    __asm__ volatile("mrs %0, tpidr_el0" : "=r"(value));
    return value;
}

/**
 * Reads the thread pointer libnx recorded, which `__aarch64_read_tp()` returns.
 * The register above has to agree with it.
 */
static inline uint64_t read_recorded_tls_ptr(void) {
    return *(volatile uint64_t*)((uint8_t*)armGetTls() + THREAD_VARS_TLS_PTR_OFFSET);
}

static inline void threadSleepMs(int64_t ms) {
    svcSleepThread(ms * 1000000);
}

typedef struct {
    /** This worker's thread pointer, for the distinctness check. */
    uint64_t tp;
    /** Non-zero if `tpidr_el0` disagreed with the recorded pointer. */
    int mismatched;
    /** Non-zero if `tpidr_el0` changed across a sleep. */
    int lost_across_switch;
} worker_result_t;

/**
 * Checks the register against the recorded pointer, sleeps long enough to be
 * descheduled and rescheduled, then checks both again.
 *
 * The sleep is the point of the test. A `tpidr_el0` the kernel does not treat
 * as per-thread state would pass the checks either side of it and fail here.
 */
static int worker(void* arg) {
    worker_result_t* out = (worker_result_t*)arg;

    out->tp = read_tpidr_el0();
    out->mismatched = (out->tp != read_recorded_tls_ptr());

    threadSleepMs(YIELD_SLEEP_MS);

    const uint64_t after = read_tpidr_el0();
    out->lost_across_switch = (after != out->tp) || (after != read_recorded_tls_ptr());

    return 0;
}

test_rc_t test_0003_thread_pointer_is_per_thread_and_preserved(void) {
    // The main thread first: nx-rt-core sets its pointer, which is a different
    // path from the workers below.
    const uint64_t main_tp = read_tpidr_el0();
    if (main_tp == 0 || main_tp != read_recorded_tls_ptr()) {
        return TEST_ASSERTION_FAILED;
    }

    thrd_t workers[WORKERS];
    worker_result_t results[WORKERS];

    for (int i = 0; i < WORKERS; i++) {
        results[i].tp = 0;
        results[i].mismatched = 0;
        results[i].lost_across_switch = 0;

        if (thrd_create(&workers[i], worker, &results[i]) != thrd_success) {
            return TEST_ASSERTION_FAILED;
        }
    }

    for (int i = 0; i < WORKERS; i++) {
        if (thrd_join(workers[i], NULL) != thrd_success) {
            return TEST_ASSERTION_FAILED;
        }
    }

    for (int i = 0; i < WORKERS; i++) {
        if (results[i].tp == 0 || results[i].mismatched || results[i].lost_across_switch) {
            return TEST_ASSERTION_FAILED;
        }

        // Each thread needs its own block: one shared pointer would make every
        // thread-local alias, which the per-thread checks above cannot see.
        if (results[i].tp == main_tp) {
            return TEST_ASSERTION_FAILED;
        }
        for (int j = i + 1; j < WORKERS; j++) {
            if (results[i].tp == results[j].tp) {
                return TEST_ASSERTION_FAILED;
            }
        }
    }

    // Still intact on this thread after the workers ran and exited.
    if (read_tpidr_el0() != main_tp) {
        return TEST_ASSERTION_FAILED;
    }

    return TEST_SUCCESS;
}

//</editor-fold>
