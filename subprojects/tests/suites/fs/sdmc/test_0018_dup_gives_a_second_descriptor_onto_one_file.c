#include <fcntl.h>
#include <string.h>
#include <unistd.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0018: dup gives a second descriptor onto one file">

#define TEST_DIR SDMC_TEST_DIR("0018-dup-gives-a-second-descriptor-onto-one-file")
#define SHARED_PATH TEST_DIR "/shared.txt"

test_rc_t test_0018_dup_gives_a_second_descriptor_onto_one_file(void)
{
    //* Given
    // One descriptor on an empty file, and a duplicate of it. Unlike two
    // separate opens, the two descriptors name one open file and so share the
    // one position: what one writes, the other continues after.
    if (!sdmc_fixture_open(TEST_DIR)) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }

    const int original = open(SHARED_PATH, O_CREAT | O_WRONLY | O_TRUNC, 0777);
    if (original < 0) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }

    //* When
    // Half the content goes through each descriptor.
    const int duplicate = dup(original);
    const bool duplicated = duplicate >= 0 && duplicate != original;

    const bool written = duplicated
        && write(original, "ABC", 3) == 3
        && write(duplicate, "DEF", 3) == 3;

    const bool closed = (!duplicated || close(duplicate) == 0) && close(original) == 0;

    //* Then
    // The two writes sit one after the other, which they only do if the second
    // descriptor picked up the position the first left behind. Had it opened
    // its own file object, the second write would have started at zero and
    // overwritten the first.
    char content[8] = {0};
    const bool read_back = sdmc_read_file(SHARED_PATH, content, sizeof(content));

    const bool correct = duplicated
        && written
        && closed
        && read_back
        && strcmp(content, "ABCDEF") == 0;

    sdmc_fixture_close(TEST_DIR);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
