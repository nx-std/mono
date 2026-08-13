#include <string.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0010: a non-ASCII name resolves">

test_rc_t test_0010_a_non_ascii_name_resolves(void)
{
    //* Given
    // The image mounted, holding a directory and a file whose names are not
    // ASCII. An image stores a name as the bytes the builder wrote, and a
    // lookup compares those bytes, so nothing on this route should be reading
    // them as characters.
    if (!romfs_fixture_open()) {
        return TEST_SETUP_FAILED;
    }

    //* When
    // The file is opened by its own name, which the walk has to split on `/`
    // and match without decoding what sits between the slashes.
    char buf[64];
    const bool read_ok = romfs_read_file(ROMFS_NON_ASCII_PATH, buf, sizeof(buf));

    //* Then
    // The name matched and the bytes came back.
    const bool correct = read_ok && strcmp(buf, ROMFS_NON_ASCII_CONTENT) == 0;

    romfs_fixture_close();
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
