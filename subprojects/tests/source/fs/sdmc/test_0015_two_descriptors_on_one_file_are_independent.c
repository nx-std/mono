#include <stdio.h>

#include <switch.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0015: two descriptors on one file are independent">

#define TEST_DIR SDMC_TEST_DIR("0015-two-descriptors-on-one-file-are-independent")
#define SHARED_PATH TEST_DIR "/shared.txt"
#define CONTENT "ABCDEF"

test_rc_t test_0015_two_descriptors_on_one_file_are_independent(void)
{
    //* Given
    // One file, opened twice. Each descriptor owns its own file object and its
    // own position; sharing either of them would show up as one read moving
    // the other's cursor.
    if (!sdmc_fixture_open(TEST_DIR) || !sdmc_write_file(SHARED_PATH, CONTENT)) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }

    FILE* first = fopen(SHARED_PATH, "r");
    FILE* second = fopen(SHARED_PATH, "r");
    if (first == NULL || second == NULL) {
        if (first != NULL) {
            fclose(first);
        }
        if (second != NULL) {
            fclose(second);
        }
        sdmc_fixture_close(TEST_DIR);
        return TEST_ASSERTION_FAILED;
    }

    //* When
    // The first descriptor reads three bytes; the second reads one.
    char head[4] = {0};
    const size_t first_read = fread(head, 1, 3, first);
    const int second_byte = fgetc(second);
    const int first_byte = fgetc(first);

    const bool closed = fclose(first) == 0 && fclose(second) == 0;

    //* Then
    // The second descriptor started at the beginning regardless of what the
    // first had read, and the first carried on from where it left off.
    const bool correct = closed
        && first_read == 3
        && head[0] == 'A'
        && head[2] == 'C'
        && second_byte == 'A'
        && first_byte == 'D';

    sdmc_fixture_close(TEST_DIR);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
