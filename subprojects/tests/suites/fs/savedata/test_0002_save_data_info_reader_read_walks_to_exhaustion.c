#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0002: save data info reader read walks to exhaustion">

test_rc_t test_0002_save_data_info_reader_read_walks_to_exhaustion(void)
{
    //* Given
    // A reader over the user savedata space.
    FsSaveDataInfoReader reader;
    if (R_FAILED(fsOpenSaveDataInfoReader(&reader, FsSaveDataSpaceId_User))) {
        return TEST_SETUP_FAILED;
    }

    //* When
    // The reader is walked one entry at a time until it reports none left.
    const s64 count = savedata_drain_reader(&reader);

    //* Then
    // The walk ended rather than running forever, and every read stayed inside
    // the one-entry buffer it was given. A console with no user savedata is a
    // valid answer here: what is under test is that the walk terminates.
    fsSaveDataInfoReaderClose(&reader);
    return count >= 0 ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
