#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0007: open save data for an unknown application fails">

test_rc_t test_0007_open_save_data_for_an_unknown_application_fails(void)
{
    //* Given
    // An application id no title owns, and the common user.
    const AccountUid uid = {0};

    //* When
    FsFileSystem fs;
    const Result rc = fsOpen_SaveData(&fs, SAVEDATA_UNKNOWN_APPLICATION_ID, uid);

    //* Then
    // The request reached the server and came back refused. Returning at all is
    // half of what is under test: a command left to libnx would park on a
    // condvar instead, with no code to report.
    if (R_SUCCEEDED(rc)) {
        fsFsClose(&fs);
        return TEST_ASSERTION_FAILED;
    }

    return TEST_SUCCESS;
}

//</editor-fold>
