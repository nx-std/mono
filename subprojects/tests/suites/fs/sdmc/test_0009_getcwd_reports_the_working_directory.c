#include <limits.h>
#include <string.h>
#include <unistd.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0009: getcwd reports the working directory">

#define TEST_DIR SDMC_TEST_DIR("0009-getcwd-reports-the-working-directory")
test_rc_t test_0009_getcwd_reports_the_working_directory(void)
{
    //* Given
    // A directory to move into, which has to exist: the device checks that the
    // path names a directory before it accepts it.
    if (!sdmc_fixture_open(TEST_DIR)) {
        return TEST_SETUP_FAILED;
    }

    //* When
    const bool entered = chdir(TEST_DIR) == 0;

    //* Then
    // The change was accepted and is what gets reported back afterwards. A
    // device that refused the directory would fail the `chdir` instead, which
    // is the failure this test is really watching for.
    char cwd[PATH_MAX] = {0};
    const bool correct = entered
        && getcwd(cwd, sizeof(cwd)) != NULL
        && strcmp(cwd, TEST_DIR) == 0;

    chdir("sdmc:/");
    sdmc_fixture_close(TEST_DIR);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
