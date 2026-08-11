#include <stdio.h>
#include <string.h>
#include <unistd.h>

#include <switch.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0008: chdir makes relative paths resolve">

#define TEST_DIR SDMC_TEST_DIR("0008-chdir-makes-relative-paths-resolve")
#define RELATIVE_NAME "relative.txt"
#define ABSOLUTE_PATH TEST_DIR "/" RELATIVE_NAME
#define CONTENT "resolved"

test_rc_t test_0008_chdir_makes_relative_paths_resolve(void)
{
    //* Given
    // An empty fixture directory, made the working directory.
    if (!sdmc_fixture_open(TEST_DIR) || chdir(TEST_DIR) != 0) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }

    //* When
    // A file is written and read back by a name carrying neither device nor
    // directory.
    const bool wrote = sdmc_write_file(RELATIVE_NAME, CONTENT);
    char buf[32] = {0};
    const bool read_back = sdmc_read_file(RELATIVE_NAME, buf, sizeof(buf));

    //* Then
    // The relative name reached the same file the absolute one names: the
    // device joined it onto the directory it was told to work in.
    char absolute[32] = {0};
    const bool correct = wrote
        && read_back
        && strcmp(buf, CONTENT) == 0
        && sdmc_read_file(ABSOLUTE_PATH, absolute, sizeof(absolute))
        && strcmp(absolute, CONTENT) == 0;

    // The working directory belongs to the device and outlives this call, so
    // the tests after this one start where they expect.
    chdir("sdmc:/");
    sdmc_fixture_close(TEST_DIR);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
