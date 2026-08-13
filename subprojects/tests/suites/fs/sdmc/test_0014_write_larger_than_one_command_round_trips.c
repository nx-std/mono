#include <stdio.h>
#include <stdlib.h>
#include <sys/stat.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0014: a write larger than one command round trips">

#define TEST_DIR SDMC_TEST_DIR("0014-write-larger-than-one-command-round-trips")
#define LARGE_PATH TEST_DIR "/large.bin"

/**
 * @brief Size of the payload, in bytes.
 *
 * Comfortably past what fits the IPC buffer a command is built in, so the
 * transfer has to go through a buffer descriptor rather than riding along
 * inside the request.
 */
#define LARGE_LEN (64 * 1024)

test_rc_t test_0014_write_larger_than_one_command_round_trips(void)
{
    //* Given
    // A payload whose every byte depends on where it sits, so a transfer that
    // dropped, duplicated or reordered a stretch of it cannot pass.
    if (!sdmc_fixture_open(TEST_DIR)) {
        return TEST_SETUP_FAILED;
    }

    uint8_t* written = malloc(LARGE_LEN);
    uint8_t* read_back = malloc(LARGE_LEN);
    if (written == NULL || read_back == NULL) {
        free(written);
        free(read_back);
        sdmc_fixture_close(TEST_DIR);
        return TEST_ASSERTION_FAILED;
    }

    for (size_t i = 0; i < LARGE_LEN; i++) {
        written[i] = (uint8_t)(i * 31 + (i >> 8));
    }

    //* When
    FILE* out = fopen(LARGE_PATH, "wb");
    const bool wrote = out != NULL
        && fwrite(written, 1, LARGE_LEN, out) == LARGE_LEN;
    const bool closed_out = out != NULL && fclose(out) == 0;

    FILE* in = fopen(LARGE_PATH, "rb");
    const bool read = in != NULL
        && fread(read_back, 1, LARGE_LEN, in) == LARGE_LEN;
    const bool closed_in = in != NULL && fclose(in) == 0;

    //* Then
    // Every byte came back where it went in, and the file is exactly as long
    // as what was written.
    struct stat st;
    const bool correct = wrote
        && closed_out
        && read
        && closed_in
        && memcmp(written, read_back, LARGE_LEN) == 0
        && stat(LARGE_PATH, &st) == 0
        && st.st_size == LARGE_LEN;

    free(written);
    free(read_back);

    sdmc_fixture_close(TEST_DIR);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
