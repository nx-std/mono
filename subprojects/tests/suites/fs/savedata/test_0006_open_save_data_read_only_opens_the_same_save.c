#include <switch.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0006: open save data read only opens the same save">

test_rc_t test_0006_open_save_data_read_only_opens_the_same_save(void)
{
    //* Given
    // The same account savedata test 0005 opens. The read-only opener takes a
    // different command, which arrived in HOS 2.0.0.
    if (hosversionBefore(2, 0, 0)) {
        return TEST_SKIPPED;
    }

    FsSaveDataInfo info;
    if (!savedata_find_account_save(&info)) {
        return TEST_SKIPPED;
    }

    //* When
    FsFileSystem fs;
    const Result rc = fsOpen_SaveDataReadOnly(&fs, info.application_id, info.uid);

    //* Then
    // The filesystem opened, and its root resolves through the read-only view
    // just as it does through the writable one.
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
