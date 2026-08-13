#include <stdio.h>
#include <string.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0003: seek moves the read position">

#define TEST_DIR SDMC_TEST_DIR("0003-seek-moves-the-read-position")
#define FILE_PATH TEST_DIR "/seek.txt"
#define CONTENT "0123456789"

test_rc_t test_0003_seek_moves_the_read_position(void)
{
    //* Given
    // A file whose every byte says where it sits, so a read proves which
    // position it started from.
    if (!sdmc_fixture_open(TEST_DIR) || !sdmc_write_file(FILE_PATH, CONTENT)) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }

    FILE* file = fopen(FILE_PATH, "r");
    if (file == NULL) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_ASSERTION_FAILED;
    }

    //* When
    // The position is moved from each of the three origins the device is asked
    // to understand, and one byte is read back after each.
    char from_start = 0;
    const bool start_ok = fseek(file, 4, SEEK_SET) == 0
        && ftell(file) == 4
        && fread(&from_start, 1, 1, file) == 1;

    // The read above advanced the position to 5, so this steps back to 2.
    char from_current = 0;
    const bool current_ok = fseek(file, -3, SEEK_CUR) == 0
        && ftell(file) == 2
        && fread(&from_current, 1, 1, file) == 1;

    char from_end = 0;
    const bool end_ok = fseek(file, -1, SEEK_END) == 0
        && ftell(file) == (long)strlen(CONTENT) - 1
        && fread(&from_end, 1, 1, file) == 1;

    const bool closed = fclose(file) == 0;

    //* Then
    // Each read returned the byte that sits at the position asked for.
    const bool correct = start_ok && current_ok && end_ok && closed
        && from_start == '4'
        && from_current == '2'
        && from_end == '9';

    sdmc_fixture_close(TEST_DIR);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
