#include "ledger.h"

#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>

// For the directory the runner keeps its files in: the programs it receives and
// this run's results live in the same place.
#include "../rig.h"

/** @brief The file the run is kept in between the runner's own lifetimes. */
#define LEDGER_PATH RIG_DIR "/results.log"

/**
 * @brief Writes the run out, so the next start of the runner can read it back.
 *
 * A run that cannot be written is still shown for as long as the runner is up;
 * there is nowhere better to report the failure than the screen it would be
 * competing with, and losing the file costs the run its history, not its
 * results.
 */
static void ledger_save(const Ledger* ledger)
{
    if (mkdir(RIG_DIR, 0777) != 0 && errno != EEXIST) {
        return;
    }

    FILE* file = fopen(LEDGER_PATH, "w");
    if (file == NULL) {
        return;
    }

    fprintf(file, "%d\n", ledger->dropped);
    for (int i = 0; i < ledger->count; i++) {
        const LedgerEntry* entry = &ledger->entries[i];
        fprintf(file, "%s %d %d %d\n", entry->suite, entry->passed, entry->failed, entry->skipped);
    }

    fclose(file);
}

void ledger_load(Ledger* ledger)
{
    memset(ledger, 0, sizeof(*ledger));

    FILE* file = fopen(LEDGER_PATH, "r");
    if (file == NULL) {
        return;
    }

    if (fscanf(file, "%d", &ledger->dropped) != 1 || ledger->dropped < 0) {
        // The file is not one we wrote, or not one we finished writing. An
        // unreadable run is an empty one: it is a record of tests, not a test
        // result itself.
        fclose(file);
        memset(ledger, 0, sizeof(*ledger));
        return;
    }

    while (ledger->count < LEDGER_CAPACITY) {
        LedgerEntry* entry = &ledger->entries[ledger->count];
        if (fscanf(file, "%23s %d %d %d", entry->suite, &entry->passed, &entry->failed,
                   &entry->skipped)
            != 4) {
            break;
        }
        ledger->count++;
    }

    fclose(file);
}

LedgerTotals ledger_totals(const Ledger* ledger)
{
    LedgerTotals totals = { .passed = 0, .failed = 0, .skipped = 0 };

    for (int i = 0; i < ledger->count; i++) {
        totals.passed += ledger->entries[i].passed;
        totals.failed += ledger->entries[i].failed;
        totals.skipped += ledger->entries[i].skipped;
    }

    return totals;
}

void ledger_clear(Ledger* ledger)
{
    memset(ledger, 0, sizeof(*ledger));

    // A run with nothing recorded yet has no file to remove, which is the usual
    // case rather than a failure, so the result is not worth looking at.
    (void)remove(LEDGER_PATH);
}

bool ledger_record(Ledger* ledger, const char* report)
{
    LedgerEntry entry;
    memset(&entry, 0, sizeof(entry));

    if (sscanf(report, "%23[^:]:%d:%d:%d", entry.suite, &entry.passed, &entry.failed,
               &entry.skipped)
        != 4) {
        return false;
    }

    if (ledger->count >= LEDGER_CAPACITY) {
        ledger->dropped++;
    } else {
        ledger->entries[ledger->count] = entry;
        ledger->count++;
    }

    ledger_save(ledger);
    return true;
}
