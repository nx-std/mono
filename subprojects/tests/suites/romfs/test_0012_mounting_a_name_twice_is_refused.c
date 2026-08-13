#include <stdio.h>
#include <string.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0012: mounting a name twice is refused">

test_rc_t test_0012_mounting_a_name_twice_is_refused(void)
{
    //* Given
    // The image already mounted under the default name.
    if (!romfs_fixture_open()) {
        return TEST_SETUP_FAILED;
    }

    //* When
    // The same name is mounted again.
    const Result rc = romfsInit();

    //* Then
    // The second mount was refused, and the first is untouched. Refusing rather
    // than replacing is what a caller checking for a name collision relies on,
    // and a second mount that silently took over would strand every descriptor
    // already open on the first.
    //
    // This case fails against libnx by construction rather than by regression:
    // its romfs mount never looks at whether the name is taken, and the
    // registration that follows replaces the first device instead of being
    // rejected. libnx's own filesystem device does check, which is the side
    // this crate takes.
    char buf[64];
    const bool intact = romfs_read_file(ROMFS_HELLO_PATH, buf, sizeof(buf))
        && strcmp(buf, ROMFS_HELLO_CONTENT) == 0;
    const bool correct = R_FAILED(rc) && intact;

    romfs_fixture_close();
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
