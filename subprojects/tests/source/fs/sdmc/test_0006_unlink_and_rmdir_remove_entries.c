#include <sys/stat.h>
#include <unistd.h>

#include <switch.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0006: unlink and rmdir remove entries">

#define FILE_PATH SDMC_PATH("doomed.txt")
#define DIR_PATH SDMC_PATH("doomed-dir")

test_rc_t test_0006_unlink_and_rmdir_remove_entries(void)
{
    //* Given
    // One file and one empty directory, which are the two things the device is
    // asked to remove and which it removes through different commands.
    if (!sdmc_fixture_reset()
        || !sdmc_write_file(FILE_PATH, "delete me")
        || mkdir(DIR_PATH, 0777) != 0) {
        sdmc_remove_tree(SDMC_ROOT);
        return TEST_ASSERTION_FAILED;
    }

    //* When
    const bool file_removed = unlink(FILE_PATH) == 0;
    const bool dir_removed = rmdir(DIR_PATH) == 0;

    //* Then
    // Neither path resolves to anything afterwards.
    struct stat st;
    const bool file_gone = stat(FILE_PATH, &st) != 0;
    const bool dir_gone = stat(DIR_PATH, &st) != 0;

    const bool correct = file_removed && dir_removed && file_gone && dir_gone;

    sdmc_remove_tree(SDMC_ROOT);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
