#include <string.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0002: reading a file returns the bundled bytes">

test_rc_t test_0002_reading_a_file_returns_the_bundled_bytes(void)
{
    //* Given
    // The image mounted, holding a file whose contents were fixed at build time.
    if (!romfs_fixture_open()) {
        return TEST_SETUP_FAILED;
    }

    //* When
    // The file is read through stdio, the route a homebrew binary actually
    // takes: newlib to libsysbase to the descriptor table to the device.
    char buf[64];
    const bool read_ok = romfs_read_file(ROMFS_HELLO_PATH, buf, sizeof(buf));

    //* Then
    // Every byte came back, in order, and nothing was appended.
    const bool correct = read_ok && strcmp(buf, ROMFS_HELLO_CONTENT) == 0;

    romfs_fixture_close();
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
