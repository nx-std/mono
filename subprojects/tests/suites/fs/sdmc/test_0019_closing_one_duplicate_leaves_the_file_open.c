#include <fcntl.h>
#include <string.h>
#include <unistd.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0019: closing one duplicate leaves the file open">

#define TEST_DIR SDMC_TEST_DIR("0019-closing-one-duplicate-leaves-the-file-open")
#define SHARED_PATH TEST_DIR "/shared.txt"

test_rc_t test_0019_closing_one_duplicate_leaves_the_file_open(void)
{
    //* Given
    // Two descriptors on one open file, one of them about to be closed. The
    // file belongs to both of them, so the close frees a descriptor number and
    // nothing else.
    if (!sdmc_fixture_open(TEST_DIR)) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }

    const int original = open(SHARED_PATH, O_CREAT | O_WRONLY | O_TRUNC, 0777);
    if (original < 0) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }

    const int duplicate = dup(original);
    if (duplicate < 0 || write(original, "ABC", 3) != 3) {
        close(original);
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }

    //* When
    // The descriptor that opened the file is closed, and the survivor writes on.
    const bool first_closed = close(original) == 0;
    const bool written = write(duplicate, "DEF", 3) == 3;
    const bool second_closed = close(duplicate) == 0;

    //* Then
    // Both writes are there. A close that told the device to close the file
    // rather than counting the descriptors still on it would have left the
    // survivor writing to something the device had already let go of.
    char content[8] = {0};
    const bool read_back = sdmc_read_file(SHARED_PATH, content, sizeof(content));

    const bool correct = first_closed
        && written
        && second_closed
        && read_back
        && strcmp(content, "ABCDEF") == 0;

    sdmc_fixture_close(TEST_DIR);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
