#include <stdio.h>
#include <string.h>

#include <switch.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0012: append mode writes at the end">

#define TEST_DIR SDMC_TEST_DIR("0012-append-mode-writes-at-the-end")
#define APPEND_PATH TEST_DIR "/append.txt"
#define HEAD "head"
#define TAIL "tail"

test_rc_t test_0012_append_mode_writes_at_the_end(void)
{
    //* Given
    // A file with known contents, opened for appending. The position an append
    // writes at is the file's own end, not wherever the descriptor happens to
    // be, which is what makes this different from an ordinary write.
    if (!sdmc_fixture_open(TEST_DIR) || !sdmc_write_file(APPEND_PATH, HEAD)) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }

    //* When
    FILE* file = fopen(APPEND_PATH, "a");
    if (file == NULL) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_ASSERTION_FAILED;
    }

    // Seeking to the start is deliberate: an appending descriptor must ignore
    // it and still write at the end.
    fseek(file, 0, SEEK_SET);
    const size_t written = fwrite(TAIL, 1, strlen(TAIL), file);
    const bool closed = fclose(file) == 0;

    //* Then
    // Both strings are there, in the order they were written, so the append
    // extended the file rather than overwriting its first bytes.
    char buf[32] = {0};
    const bool correct = written == strlen(TAIL)
        && closed
        && sdmc_read_file(APPEND_PATH, buf, sizeof(buf))
        && strcmp(buf, HEAD TAIL) == 0;

    sdmc_fixture_close(TEST_DIR);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
