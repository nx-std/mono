#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0001: open save data info reader over the user space succeeds">

test_rc_t test_0001_open_save_data_info_reader_over_the_user_space_succeeds(void)
{
    //* When
    // A reader is opened over the user savedata space, which is the command the
    // `save` example reaches first.
    FsSaveDataInfoReader reader;
    const Result rc = fsOpenSaveDataInfoReader(&reader, FsSaveDataSpaceId_User);

    //* Then
    // The server answered with an object, and closing it is accepted. A command
    // that fell through to libnx would never return at all.
    if (R_FAILED(rc)) {
        return TEST_ASSERTION_FAILED;
    }

    fsSaveDataInfoReaderClose(&reader);
    return TEST_SUCCESS;
}

//</editor-fold>
