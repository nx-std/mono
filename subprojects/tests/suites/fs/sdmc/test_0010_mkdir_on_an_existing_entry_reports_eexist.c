#include <errno.h>
#include <sys/stat.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0010: mkdir on an existing entry reports EEXIST">

#define TEST_DIR SDMC_TEST_DIR("0010-mkdir-on-an-existing-entry-reports-eexist")
#define NESTED_DIR TEST_DIR "/nested"

test_rc_t test_0010_mkdir_on_an_existing_entry_reports_eexist(void)
{
    //* Given
    // A directory that is already there. The error number is cleared first,
    // because a successful call may leave whatever an earlier failure put
    // there.
    if (!sdmc_fixture_open(TEST_DIR) || mkdir(NESTED_DIR, 0777) != 0) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }
    errno = 0;

    //* When
    const int again = mkdir(NESTED_DIR, 0777);

    //* Then
    // The second create failed, and said the entry was already there rather
    // than reporting a generic I/O error. A caller creating a directory it may
    // already own branches on exactly this.
    const bool correct = again != 0 && errno == EEXIST;

    sdmc_fixture_close(TEST_DIR);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
