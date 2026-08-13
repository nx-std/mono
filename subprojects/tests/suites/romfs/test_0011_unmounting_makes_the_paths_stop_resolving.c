#include <stdio.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0011: unmounting makes the paths stop resolving">

test_rc_t test_0011_unmounting_makes_the_paths_stop_resolving(void)
{
    //* Given
    // The image mounted, and a path under it that resolves.
    if (!romfs_fixture_open()) {
        return TEST_SETUP_FAILED;
    }

    FILE* before = fopen(ROMFS_HELLO_PATH, "r");
    const bool resolved_before = before != NULL;
    if (before != NULL) {
        fclose(before);
    }

    //* When
    // The image is given back.
    const Result rc = romfsExit();

    //* Then
    // The name stopped resolving, and mounting again brings it back: unmounting
    // empties the device rather than retiring the name, and a device that could
    // not be refilled would leave the second mount with nowhere to go.
    FILE* after = fopen(ROMFS_HELLO_PATH, "r");
    const bool resolved_after = after != NULL;
    if (after != NULL) {
        fclose(after);
    }

    const bool remounts = romfs_fixture_open();
    FILE* again = fopen(ROMFS_HELLO_PATH, "r");
    const bool resolves_again = again != NULL;
    if (again != NULL) {
        fclose(again);
    }

    const bool correct = resolved_before && R_SUCCEEDED(rc) && !resolved_after
        && remounts && resolves_again;

    romfs_fixture_close();
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
