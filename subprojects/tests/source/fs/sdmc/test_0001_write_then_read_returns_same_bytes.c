#include <string.h>

#include <switch.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0001: write then read returns same bytes">

#define FILE_PATH SDMC_PATH("roundtrip.txt")
#define CONTENT "nx-std filesystem round trip"

test_rc_t test_0001_write_then_read_returns_same_bytes(void)
{
    //* Given
    // An empty fixture directory, so nothing from an earlier run can satisfy
    // the read that follows.
    if (!sdmc_fixture_reset()) {
        return TEST_ASSERTION_FAILED;
    }

    //* When
    // The bytes go out through `fopen`/`fwrite`/`fclose` and come back through
    // `fopen`/`fread`/`fclose`, which is the whole devoptab transfer path.
    if (!sdmc_write_file(FILE_PATH, CONTENT)) {
        sdmc_remove_tree(SDMC_ROOT);
        return TEST_ASSERTION_FAILED;
    }

    char buf[128];
    const bool read_ok = sdmc_read_file(FILE_PATH, buf, sizeof(buf));

    //* Then
    // What came back is byte-for-byte what went in.
    const bool matches = read_ok && strcmp(buf, CONTENT) == 0;

    sdmc_remove_tree(SDMC_ROOT);
    return matches ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
