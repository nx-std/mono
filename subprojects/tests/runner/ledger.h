#pragma once

#include <stdbool.h>

/** @brief How much room a suite's name gets in the table. */
#define LEDGER_SUITE_SIZE 24

/**
 * @brief How many suites one run can record.
 *
 * A run past this keeps going and counts what it could not record, so the
 * screen says the table is short rather than quietly showing a partial run as
 * if it were the whole one.
 */
#define LEDGER_CAPACITY 12

/** @brief What one suite reported. */
typedef struct {
    /** The name the suite gave for itself. */
    char suite[LEDGER_SUITE_SIZE];
    /** Cases that returned success. */
    int passed;
    /** Cases that failed, including those whose fixture could not be built. */
    int failed;
    /** Cases that were skipped or are not written yet. */
    int skipped;
} LedgerEntry;

/**
 * @brief Every suite of the current run, in the order they reported.
 *
 * The runner does not survive the suites it launches: each one replaces it, and
 * it starts again afterwards with nothing it had before. So the run's results
 * live on the SD card, and this is that file held in memory.
 */
typedef struct {
    LedgerEntry entries[LEDGER_CAPACITY];
    /** How many entries are filled. */
    int count;
    /** How many suites reported once the table was already full. */
    int dropped;
} Ledger;

/** @brief What a whole run came to. */
typedef struct {
    int passed;
    int failed;
    int skipped;
} LedgerTotals;

/**
 * @brief Reads the run in progress, if there is one.
 *
 * A ledger with no file behind it loads empty, which is what a first run is.
 */
void ledger_load(Ledger* ledger);

/**
 * @brief Adds up every suite recorded so far.
 *
 * Suites the table had no room for are missing from this, the same way they are
 * missing from the table; `dropped` is what says so.
 */
LedgerTotals ledger_totals(const Ledger* ledger);

/**
 * @brief Ends the current run and starts an empty one.
 *
 * Called when the runner is started by hand rather than handed back to: the
 * results on screen then belong to the run being started, not to whatever ran
 * before it.
 */
void ledger_clear(Ledger* ledger);

/**
 * @brief Records what a suite reported and saves the run.
 *
 * @param report The text following `HANDBACK_RESULT_PREFIX`.
 * @return `false` when the report could not be read, in which case nothing is
 *         recorded.
 */
bool ledger_record(Ledger* ledger, const char* report);
