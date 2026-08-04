#include <string.h>
#include <sys/stat.h>

#include <switch.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0002: stat reports size and type">

#define FILE_PATH SDMC_PATH("stat.txt")
#define CONTENT "twenty-four characters!!"

test_rc_t test_0002_stat_reports_size_and_type(void)
{
    //* Given
    // A file of known length, and the directory holding it.
    if (!sdmc_fixture_reset() || !sdmc_write_file(FILE_PATH, CONTENT)) {
        sdmc_remove_tree(SDMC_ROOT);
        return TEST_ASSERTION_FAILED;
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
    const bool dir_ok = stat(SDMC_ROOT, &dir_stat) == 0;

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

    sdmc_remove_tree(SDMC_ROOT);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
