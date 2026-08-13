#include <stdio.h>
#include <string.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0008: opening for writing is refused">

test_rc_t test_0008_opening_for_writing_is_refused(void)
{
    //* Given
    // The image mounted, holding a file with known contents.
    if (!romfs_fixture_open()) {
        return TEST_SETUP_FAILED;
    }

    //* When
    // A file that exists is opened for writing, and one that does not is opened
    // for creation. Both have to be refused: an image is fixed by whoever built
    // it, and there is nowhere for either to write.
    FILE* truncating = fopen(ROMFS_HELLO_PATH, "w");
    if (truncating != NULL) {
        fclose(truncating);
    }

    FILE* creating = fopen("romfs:/created.txt", "w");
    if (creating != NULL) {
        fclose(creating);
    }

    //* Then
    // Both were refused, and the file that was there is untouched. The second
    // half is what makes this worth running: a device that accepted the open
    // and dropped the writes would look the same until something read the file
    // back.
    char buf[64];
    const bool intact = romfs_read_file(ROMFS_HELLO_PATH, buf, sizeof(buf))
        && strcmp(buf, ROMFS_HELLO_CONTENT) == 0;
    const bool correct = truncating == NULL && creating == NULL && intact;

    romfs_fixture_close();
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
