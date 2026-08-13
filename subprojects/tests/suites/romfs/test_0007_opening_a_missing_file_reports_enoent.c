#include <errno.h>
#include <stdio.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0007: opening a missing file reports ENOENT">

test_rc_t test_0007_opening_a_missing_file_reports_enoent(void)
{
    //* Given
    // The image mounted, and a path under it that names nothing.
    if (!romfs_fixture_open()) {
        return TEST_SETUP_FAILED;
    }

    //* When
    // The path is opened for reading.
    errno = 0;
    FILE* file = fopen("romfs:/there-is-no-such-file.txt", "r");
    const int failure = errno;
    if (file != NULL) {
        fclose(file);
    }

    //* Then
    // The open failed with the code a caller branches on, rather than the
    // generic I/O failure a lookup that could not tell "absent" from "broken"
    // would report.
    const bool correct = file == NULL && failure == ENOENT;

    romfs_fixture_close();
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
