#include <stdio.h>
#include <string.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0005: seek moves the read position">

/** The line each seek below is aimed at, chosen to land nowhere near an edge. */
#define TARGET_LINE 100

test_rc_t test_0005_seek_moves_the_read_position(void)
{
    //* Given
    // The image mounted, and a file whose every line says which line it is, so
    // a read that landed at the wrong offset reports where it actually landed
    // instead of merely failing.
    if (!romfs_fixture_open()) {
        return TEST_SETUP_FAILED;
    }

    FILE* file = fopen(ROMFS_LINES_PATH, "r");
    if (file == NULL) {
        romfs_fixture_close();
        return TEST_SETUP_FAILED;
    }

    char expected[ROMFS_LINES_LINE_LEN + 1];
    romfs_expected_line(TARGET_LINE, expected);

    //* When
    // The same line is reached three ways: from the start, from the current
    // position after a read, and backwards from the end.
    char from_start[ROMFS_LINES_LINE_LEN + 1] = {0};
    const bool start_ok =
        fseek(file, (long)(TARGET_LINE * ROMFS_LINES_LINE_LEN), SEEK_SET) == 0
        && fread(from_start, 1, ROMFS_LINES_LINE_LEN, file) == ROMFS_LINES_LINE_LEN;

    // The read above left the position at the start of the next line, so one
    // line backwards is the line just read.
    char from_current[ROMFS_LINES_LINE_LEN + 1] = {0};
    const bool current_ok =
        fseek(file, -(long)ROMFS_LINES_LINE_LEN, SEEK_CUR) == 0
        && fread(from_current, 1, ROMFS_LINES_LINE_LEN, file) == ROMFS_LINES_LINE_LEN;

    const long from_end_offset =
        -(long)((ROMFS_LINES_COUNT - TARGET_LINE) * ROMFS_LINES_LINE_LEN);
    char from_end[ROMFS_LINES_LINE_LEN + 1] = {0};
    const bool end_ok = fseek(file, from_end_offset, SEEK_END) == 0
        && fread(from_end, 1, ROMFS_LINES_LINE_LEN, file) == ROMFS_LINES_LINE_LEN;

    //* Then
    // All three landed on the same line, which is the one the offsets name.
    const bool correct = start_ok && current_ok && end_ok
        && strcmp(from_start, expected) == 0
        && strcmp(from_current, expected) == 0
        && strcmp(from_end, expected) == 0;

    fclose(file);
    romfs_fixture_close();
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
