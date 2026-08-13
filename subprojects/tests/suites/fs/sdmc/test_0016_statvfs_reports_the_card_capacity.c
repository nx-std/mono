#include <sys/statvfs.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0016: statvfs reports the card capacity">

#define TEST_DIR SDMC_TEST_DIR("0016-statvfs-reports-the-card-capacity")
test_rc_t test_0016_statvfs_reports_the_card_capacity(void)
{
    //* Given
    // A path on the mounted card. Space is a property of the filesystem rather
    // than of the entry, so any path that resolves will do.
    if (!sdmc_fixture_open(TEST_DIR)) {
        return TEST_SETUP_FAILED;
    }

    //* When
    struct statvfs st;
    const bool reported = statvfs(TEST_DIR, &st) == 0;

    //* Then
    // The card has a capacity, some of it is free, and free never exceeds
    // total. The figures are in bytes, so the block size is one: the device
    // reports what the server gives it rather than inventing a block.
    const bool correct = reported
        && st.f_bsize == 1
        && st.f_blocks > 0
        && st.f_bfree <= st.f_blocks;

    sdmc_fixture_close(TEST_DIR);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
