#include <errno.h>
#include <fcntl.h>
#include <unistd.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0017: exclusive create on an existing file reports EEXIST">

#define TEST_DIR SDMC_TEST_DIR("0017-exclusive-create-on-an-existing-file")
#define TAKEN_PATH TEST_DIR "/taken.txt"
#define FREE_PATH TEST_DIR "/free.txt"

/**
 * Tests that an exclusive create refuses a path that is already taken, and
 * still creates one that is not.
 *
 * The create and the open are two commands, and only the second one's failure
 * used to be reported: a create that failed left the open to complain that the
 * path names nothing, which blames the path for not existing rather than saying
 * why it could not be made. That is what an exclusive create must never do,
 * because `EEXIST` is the whole answer a caller is asking for.
 */
test_rc_t test_0017_exclusive_create_on_an_existing_file_reports_eexist(void)
{
    //* Given
    // One path that is taken, and one that is free.
    if (!sdmc_fixture_open(TEST_DIR) || !sdmc_write_file(TAKEN_PATH, "taken")) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }
    errno = 0;

    //* When
    // Both are opened with the flags that demand the entry not already exist.
    const int refused = open(TAKEN_PATH, O_WRONLY | O_CREAT | O_EXCL, 0666);
    const int refused_errno = errno;

    const int created = open(FREE_PATH, O_WRONLY | O_CREAT | O_EXCL, 0666);

    //* Then
    // The taken path was refused, and said the entry was already there rather
    // than that it was missing. The free one was created.
    const bool correct = refused < 0
        && refused_errno == EEXIST
        && created >= 0;

    if (refused >= 0) {
        close(refused);
    }
    if (created >= 0) {
        close(created);
    }

    sdmc_fixture_close(TEST_DIR);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
