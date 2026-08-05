#include <stdbool.h>
#include <stdint.h>

#include <switch.h>

#include "../../harness.h"

//<editor-fold desc="Test 0007: remutex stays inside its own storage">

/**
 * @brief Value written after the lock, which no lock operation may disturb.
 *
 * Chosen so that a partial overwrite is still obvious: every byte differs from
 * a thread handle, a recursion count and zero.
 */
#define CANARY 0x5A5AC3C3A5A53C3Cull

/** @brief How many times the lock is taken before being released again. */
#define REENTRY_DEPTH 3

/**
 * @brief A lock with a witness immediately behind it.
 *
 * `RMutex` is two 32-bit words and an implementation is only entitled to those
 * eight bytes. One that carries a third field writes the ninth byte onward,
 * which is the caller's memory: here the canary, in real code whatever the C
 * library happened to put next. Nothing about that failure is visible at the
 * lock itself, which is why it needs a witness.
 */
typedef struct {
    RMutex lock;
    uint64_t canary;
} probe_t;

/**
 * Tests that the lock operations write only to the lock.
 *
 * - The type is the size the C library allocates for it
 * - Locking, re-entering and unlocking leave the neighbouring bytes alone
 * - The recursion count is where a C caller reads it
 */
test_rc_t test_0007_remutex_stays_inside_its_own_storage(void) {
    //* Given
    // A lock followed immediately by a known value.
    probe_t probe;
    probe.canary = CANARY;
    rmutexInit(&probe.lock);

    // The layout the C side allocates. A Rust replacement that disagrees about
    // this cannot be made to behave by any amount of care at the call sites.
    if (sizeof(RMutex) != 2 * sizeof(uint32_t)) {
        return TEST_ASSERTION_FAILED;
    }

    //* When
    // The lock is taken to depth, then fully released.
    for (int i = 0; i < REENTRY_DEPTH; i++) {
        rmutexLock(&probe.lock);
    }

    const bool counted = probe.lock.counter == REENTRY_DEPTH;
    const bool intact_while_held = probe.canary == CANARY;

    for (int i = 0; i < REENTRY_DEPTH; i++) {
        rmutexUnlock(&probe.lock);
    }

    //* Then
    // The count is what a C caller reads at its own offset, and the value behind
    // the lock is the one that was put there.
    const bool correct = counted
        && intact_while_held
        && probe.lock.counter == 0
        && probe.canary == CANARY;

    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
