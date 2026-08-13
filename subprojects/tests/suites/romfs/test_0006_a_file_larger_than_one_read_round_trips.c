#include <stdio.h>
#include <string.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0006: a file larger than one read round trips">

/** The whole file, 16 KiB of it, read in one go. */
#define FILE_LEN (ROMFS_LINES_COUNT * ROMFS_LINES_LINE_LEN)

test_rc_t test_0006_a_file_larger_than_one_read_round_trips(void)
{
    //* Given
    // The image mounted, and a file several times the size of the buffer a
    // retried read moves at a time, so a transfer that stopped at a chunk
    // boundary or restarted one is visible here and nowhere smaller.
    if (!romfs_fixture_open()) {
        return TEST_SETUP_FAILED;
    }

    static char buf[FILE_LEN];

    //* When
    // The whole file is read in a single call.
    FILE* file = fopen(ROMFS_LINES_PATH, "r");
    size_t read = 0;
    if (file != NULL) {
        read = fread(buf, 1, sizeof(buf), file);
        fclose(file);
    }

    //* Then
    // Every byte arrived, and every line is the one belonging at its offset: a
    // read that dropped or repeated a stretch would still fill the buffer, and
    // only the per-line check catches it.
    bool lines_ok = read == sizeof(buf);
    for (size_t index = 0; lines_ok && index < ROMFS_LINES_COUNT; index++) {
        char expected[ROMFS_LINES_LINE_LEN + 1];
        romfs_expected_line(index, expected);

        lines_ok = memcmp(&buf[index * ROMFS_LINES_LINE_LEN], expected,
                          ROMFS_LINES_LINE_LEN) == 0;
    }

    romfs_fixture_close();
    return lines_ok ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
