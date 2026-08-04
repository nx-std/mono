#include <errno.h>
#include <stdio.h>

#include <switch.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0007: opening a missing file reports ENOENT">

#define MISSING_PATH SDMC_PATH("not-here.txt")

test_rc_t test_0007_opening_a_missing_file_reports_enoent(void)
{
    //* Given
    // An empty fixture directory, so the path below names nothing. The error
    // number is cleared first, because a successful call may leave whatever an
    // earlier failure put there.
    if (!sdmc_fixture_reset()) {
        return TEST_ASSERTION_FAILED;
    }
    errno = 0;

    //* When
    FILE* file = fopen(MISSING_PATH, "r");

    //* Then
    // The open failed, and it said why. Reporting the failure without the
    // reason is the interesting way for this to break: it means the device's
    // "not found" reached the boundary as a generic I/O error.
    const bool correct = file == NULL && errno == ENOENT;

    if (file != NULL) {
        fclose(file);
    }

    sdmc_remove_tree(SDMC_ROOT);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
