#include <stdio.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0001: romfsInit mounts the program's own image">

test_rc_t test_0001_romfs_init_mounts_the_programs_own_image(void)
{
    //* Given
    // Nothing mounted: every test in this suite gives the image back when it
    // finishes, so the mount this one performs is the first.

    //* When
    // The image appended to this very NRO is mounted, which means finding the
    // file the loader launched and reading the asset header inside it.
    const Result rc = romfsInit();

    //* Then
    // The mount succeeded and a path under it resolves. The second half is not
    // redundant: a device registered but never filled would take the name and
    // refuse every path under it.
    bool resolves = false;
    if (R_SUCCEEDED(rc)) {
        FILE* file = fopen(ROMFS_HELLO_PATH, "r");
        resolves = file != NULL;
        if (file != NULL) {
            fclose(file);
        }
        romfsExit();
    }

    return (R_SUCCEEDED(rc) && resolves) ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
