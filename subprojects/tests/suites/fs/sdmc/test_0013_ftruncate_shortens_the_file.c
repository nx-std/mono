#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0013: ftruncate shortens the file">

#define TEST_DIR SDMC_TEST_DIR("0013-ftruncate-shortens-the-file")
#define TRUNCATE_PATH TEST_DIR "/truncate.txt"
#define CONTENT "0123456789"
#define KEPT 4

test_rc_t test_0013_ftruncate_shortens_the_file(void)
{
    //* Given
    // A file of known length, open for writing so it can be resized through
    // its descriptor.
    if (!sdmc_fixture_open(TEST_DIR) || !sdmc_write_file(TRUNCATE_PATH, CONTENT)) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }

    FILE* file = fopen(TRUNCATE_PATH, "r+");
    if (file == NULL) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_ASSERTION_FAILED;
    }

    //* When
    const bool truncated = ftruncate(fileno(file), KEPT) == 0;
    const bool synced = fsync(fileno(file)) == 0;
    const bool closed = fclose(file) == 0;

    //* Then
    // The file is the length it was cut to, and holds the bytes that were kept.
    struct stat st;
    char buf[32] = {0};
    const bool correct = truncated
        && synced
        && closed
        && stat(TRUNCATE_PATH, &st) == 0
        && st.st_size == KEPT
        && sdmc_read_file(TRUNCATE_PATH, buf, sizeof(buf))
        && strlen(buf) == KEPT
        && strncmp(buf, CONTENT, KEPT) == 0;

    sdmc_fixture_close(TEST_DIR);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
