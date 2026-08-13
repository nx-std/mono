#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0005: open save data opens an account save">

test_rc_t test_0005_open_save_data_opens_an_account_save(void)
{
    //* Given
    // An account savedata the console actually holds, found through the info
    // reader. A console with none has nothing this test can open.
    FsSaveDataInfo info;
    if (!savedata_find_account_save(&info)) {
        return TEST_SKIPPED;
    }

    //* When
    // The savedata is opened by the application id and user the reader named.
    FsFileSystem fs;
    const Result rc = fsOpen_SaveData(&fs, info.application_id, info.uid);

    //* Then
    // The filesystem opened, and its root resolves - so the object handed back
    // is one the server will take commands on, not just a non-zero id.
    if (R_FAILED(rc)) {
        return TEST_ASSERTION_FAILED;
    }

    FsDir dir;
    const bool listed = R_SUCCEEDED(
        fsFsOpenDirectory(&fs, "/", FsDirOpenMode_ReadDirs | FsDirOpenMode_ReadFiles, &dir));
    if (listed) {
        fsDirClose(&dir);
    }

    fsFsClose(&fs);
    return listed ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
