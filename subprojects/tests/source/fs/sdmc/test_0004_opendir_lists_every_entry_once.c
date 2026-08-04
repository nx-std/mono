#include <dirent.h>
#include <string.h>
#include <sys/stat.h>

#include <switch.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0004: opendir lists every entry once">

#define NESTED_DIR SDMC_PATH("nested")

test_rc_t test_0004_opendir_lists_every_entry_once(void)
{
    //* Given
    // A directory holding two files and one subdirectory, so the walk has to
    // report both kinds and get the count right.
    if (!sdmc_fixture_reset()
        || !sdmc_write_file(SDMC_PATH("alpha.txt"), "alpha")
        || !sdmc_write_file(SDMC_PATH("beta.txt"), "beta")
        || mkdir(NESTED_DIR, 0777) != 0) {
        sdmc_remove_tree(SDMC_ROOT);
        return TEST_ASSERTION_FAILED;
    }

    //* When
    // The directory is walked to exhaustion through `opendir`/`readdir`.
    DIR* dir = opendir(SDMC_ROOT);
    if (dir == NULL) {
        sdmc_remove_tree(SDMC_ROOT);
        return TEST_ASSERTION_FAILED;
    }

    int alpha_seen = 0;
    int beta_seen = 0;
    int nested_seen = 0;
    int total = 0;

    struct dirent* ent;
    while ((ent = readdir(dir)) != NULL) {
        total++;
        if (strcmp(ent->d_name, "alpha.txt") == 0) {
            alpha_seen++;
        } else if (strcmp(ent->d_name, "beta.txt") == 0) {
            beta_seen++;
        } else if (strcmp(ent->d_name, "nested") == 0 && ent->d_type == DT_DIR) {
            nested_seen++;
        }
    }

    const bool closed = closedir(dir) == 0;

    //* Then
    // Every entry was reported exactly once, the subdirectory was reported as
    // one, and nothing else appeared.
    const bool correct = closed
        && alpha_seen == 1
        && beta_seen == 1
        && nested_seen == 1
        && total == 3;

    sdmc_remove_tree(SDMC_ROOT);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
