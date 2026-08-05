#include <stdio.h>
#include <string.h>
#include <sys/stat.h>

#include <switch.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0005: rename moves the entry">

#define TEST_DIR SDMC_TEST_DIR("0005-rename-moves-the-entry")
#define OLD_PATH TEST_DIR "/before.txt"
#define NEW_PATH TEST_DIR "/after.txt"
#define CONTENT "contents survive the rename"

test_rc_t test_0005_rename_moves_the_entry(void)
{
    //* Given
    // A file under a name nothing else uses, with known contents so the move
    // can be shown to have carried them.
    if (!sdmc_fixture_open(TEST_DIR) || !sdmc_write_file(OLD_PATH, CONTENT)) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }

    //* When
    const bool renamed = rename(OLD_PATH, NEW_PATH) == 0;

    //* Then
    // The old name resolves to nothing, the new one resolves to the file, and
    // the contents came along.
    struct stat st;
    const bool old_gone = stat(OLD_PATH, &st) != 0;
    const bool new_exists = stat(NEW_PATH, &st) == 0;

    char buf[64];
    const bool contents_kept = sdmc_read_file(NEW_PATH, buf, sizeof(buf))
        && strcmp(buf, CONTENT) == 0;

    const bool correct = renamed && old_gone && new_exists && contents_kept;

    sdmc_fixture_close(TEST_DIR);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
