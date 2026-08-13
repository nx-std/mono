#include <stdio.h>
#include <string.h>
#include <sys/stat.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0003: stat reports size and type">

test_rc_t test_0003_stat_reports_size_and_type(void)
{
    //* Given
    // The image mounted, holding a file of known length and a directory.
    if (!romfs_fixture_open()) {
        return TEST_SETUP_FAILED;
    }

    //* When
    // Both the path and the descriptor are asked about the same file, which
    // reach the device through `stat` and `fstat` respectively, and a directory
    // is asked about too.
    struct stat by_path;
    const bool path_ok = stat(ROMFS_HELLO_PATH, &by_path) == 0;

    struct stat by_fd;
    FILE* file = fopen(ROMFS_HELLO_PATH, "r");
    const bool fd_ok = file != NULL && fstat(fileno(file), &by_fd) == 0;
    if (file != NULL) {
        fclose(file);
    }

    struct stat dir_stat;
    const bool dir_ok = stat(ROMFS_DATA_DIR, &dir_stat) == 0;

    //* Then
    // Both routes report the bundled length and agree it is a regular file,
    // and the directory reports itself as a directory.
    const size_t expected = strlen(ROMFS_HELLO_CONTENT);
    const bool correct = path_ok && fd_ok && dir_ok
        && (size_t)by_path.st_size == expected
        && (size_t)by_fd.st_size == expected
        && S_ISREG(by_path.st_mode)
        && S_ISREG(by_fd.st_mode)
        && S_ISDIR(dir_stat.st_mode);

    romfs_fixture_close();
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
