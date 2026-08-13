#include <dirent.h>
#include <string.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0004: opendir lists every entry once">

test_rc_t test_0004_opendir_lists_every_entry_once(void)
{
    //* Given
    // The image mounted, holding a directory with one file and one
    // subdirectory in it.
    if (!romfs_fixture_open()) {
        return TEST_SETUP_FAILED;
    }

    //* When
    // The directory is walked to exhaustion, counting what each entry is.
    DIR* dir = opendir(ROMFS_DATA_DIR);
    size_t self_entries = 0;
    size_t parent_entries = 0;
    size_t lines_entries = 0;
    size_t nested_entries = 0;
    size_t unexpected_entries = 0;

    if (dir != NULL) {
        const struct dirent* entry;
        while ((entry = readdir(dir)) != NULL) {
            if (strcmp(entry->d_name, ".") == 0) {
                self_entries++;
            } else if (strcmp(entry->d_name, "..") == 0) {
                parent_entries++;
            } else if (strcmp(entry->d_name, "lines.txt") == 0) {
                lines_entries++;
            } else if (strcmp(entry->d_name, "nested") == 0) {
                nested_entries++;
            } else {
                unexpected_entries++;
            }
        }
        closedir(dir);
    }

    //* Then
    // Every entry appeared exactly once and nothing else did. The two synthetic
    // entries are part of the contract rather than noise to be tolerated: a
    // romfs walk reports them where the filesystem device does not, and a
    // program that skips them by name breaks if they stop arriving.
    const bool correct = dir != NULL
        && self_entries == 1
        && parent_entries == 1
        && lines_entries == 1
        && nested_entries == 1
        && unexpected_entries == 0;

    romfs_fixture_close();
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
