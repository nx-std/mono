#include <switch.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0003: open save data info reader over every space succeeds">

test_rc_t test_0003_open_save_data_info_reader_over_every_space_succeeds(void)
{
    //* When
    // A reader is opened over every space at once. `FsSaveDataSpaceId_All` is
    // not a space the server knows: it selects a different command from the one
    // a real space id takes, so this is the other half of that branch.
    FsSaveDataInfoReader reader;
    const Result rc = fsOpenSaveDataInfoReader(&reader, FsSaveDataSpaceId_All);

    //* Then
    if (R_FAILED(rc)) {
        return TEST_ASSERTION_FAILED;
    }

    fsSaveDataInfoReaderClose(&reader);
    return TEST_SUCCESS;
}

//</editor-fold>
