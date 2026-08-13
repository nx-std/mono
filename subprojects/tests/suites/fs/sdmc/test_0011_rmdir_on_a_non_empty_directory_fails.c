#include <sys/stat.h>
#include <unistd.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0011: rmdir on a non-empty directory fails">

#define TEST_DIR SDMC_TEST_DIR("0011-rmdir-on-a-non-empty-directory-fails")
#define NESTED_DIR TEST_DIR "/nested"
#define NESTED_FILE NESTED_DIR "/child.txt"

test_rc_t test_0011_rmdir_on_a_non_empty_directory_fails(void)
{
    //* Given
    // A directory holding one file.
    if (!sdmc_fixture_open(TEST_DIR)
        || mkdir(NESTED_DIR, 0777) != 0
        || !sdmc_write_file(NESTED_FILE, "child")) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }

    //* When
    const int removed = rmdir(NESTED_DIR);

    //* Then
    // The removal was refused and took nothing with it. Which error number it
    // reports is deliberately not asserted: the server refuses a non-empty
    // directory with a code that neither this implementation nor libnx gives a
    // name of its own, so both report it as a plain I/O failure.
    struct stat st;
    const bool correct = removed != 0
        && stat(NESTED_DIR, &st) == 0
        && S_ISDIR(st.st_mode)
        && stat(NESTED_FILE, &st) == 0;

    sdmc_fixture_close(TEST_DIR);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
