#include <fcntl.h>
#include <string.h>
#include <unistd.h>

#include <switch.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0020: dup2 rebinds a descriptor onto another file">

#define TEST_DIR SDMC_TEST_DIR("0020-dup2-rebinds-a-descriptor-onto-another-file")
#define KEPT_PATH TEST_DIR "/kept.txt"
#define DISPLACED_PATH TEST_DIR "/displaced.txt"

test_rc_t test_0020_dup2_rebinds_a_descriptor_onto_another_file(void)
{
    //* Given
    // Two descriptors on two files, each with a byte of its own already
    // written. This is the shape a program redirecting its output has: a
    // descriptor number somebody else will write to, and the file it should
    // reach instead.
    if (!sdmc_fixture_open(TEST_DIR)) {
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }

    const int kept = open(KEPT_PATH, O_CREAT | O_WRONLY | O_TRUNC, 0777);
    const int displaced = open(DISPLACED_PATH, O_CREAT | O_WRONLY | O_TRUNC, 0777);
    if (kept < 0 || displaced < 0 || write(kept, "A", 1) != 1
        || write(displaced, "X", 1) != 1) {
        if (kept >= 0) {
            close(kept);
        }
        if (displaced >= 0) {
            close(displaced);
        }
        sdmc_fixture_close(TEST_DIR);
        return TEST_SETUP_FAILED;
    }

    //* When
    // The second number is rebound onto the first file and written through.
    const bool rebound = dup2(kept, displaced) == displaced;
    const bool written = rebound && write(displaced, "B", 1) == 1;

    const bool closed = close(displaced) == 0 && close(kept) == 0;

    //* Then
    // The write through the rebound number reached the file it now names, and
    // continued from the position that file was already at rather than from
    // zero. The file it stopped naming kept what it had, which it only does if
    // the rebinding closed it rather than abandoning it mid-write.
    char kept_content[8] = {0};
    char displaced_content[8] = {0};
    const bool read_back = sdmc_read_file(KEPT_PATH, kept_content, sizeof(kept_content))
        && sdmc_read_file(DISPLACED_PATH, displaced_content, sizeof(displaced_content));

    const bool correct = rebound
        && written
        && closed
        && read_back
        && strcmp(kept_content, "AB") == 0
        && strcmp(displaced_content, "X") == 0;

    sdmc_fixture_close(TEST_DIR);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
