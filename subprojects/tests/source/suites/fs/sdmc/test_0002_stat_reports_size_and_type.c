#include <string.h>
#include <sys/stat.h>

#include <switch.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0002: stat reports size and type">

#define TEST_DIR SDMC_TEST_DIR("0002-stat-reports-size-and-type")
#define FILE_PATH TEST_DIR "/stat.txt"
#define CONTENT "twenty-four characters!!"

test_rc_t test_0002_stat_reports_size_and_type(void)
{
    //* Given
    // A file of known length, and the directory holding it.
    if (!sdmc_fixture_open(TEST_DIR) || !sdmc_write_file(FILE_PATH, CONTENT)) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }

    //* When
    // Both the path and the descriptor are asked about the same file, which
    // reach the device through `stat` and `fstat` respectively.
    struct stat by_path;
    const bool path_ok = stat(FILE_PATH, &by_path) == 0;

    struct stat by_fd;
    FILE* file = fopen(FILE_PATH, "r");
    const bool fd_ok = file != NULL && fstat(fileno(file), &by_fd) == 0;
    if (file != NULL) {
        fclose(file);
    }

    struct stat dir_stat;
    const bool dir_ok = stat(TEST_DIR, &dir_stat) == 0;

    //* Then
    // Both routes report the written length and agree it is a regular file,
    // and the directory holding it reports itself as a directory.
    const size_t expected = strlen(CONTENT);
    const bool correct = path_ok && fd_ok && dir_ok
        && (size_t)by_path.st_size == expected
        && (size_t)by_fd.st_size == expected
        && S_ISREG(by_path.st_mode)
        && S_ISREG(by_fd.st_mode)
        && S_ISDIR(dir_stat.st_mode);

    sdmc_fixture_close(TEST_DIR);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
