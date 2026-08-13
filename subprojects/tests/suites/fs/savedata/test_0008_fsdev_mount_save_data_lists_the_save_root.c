#include <dirent.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0008: fsdev mount save data lists the save root">

test_rc_t test_0008_fsdev_mount_save_data_lists_the_save_root(void)
{
    //* Given
    // An account savedata the console holds. This is the path the `save`
    // example takes: the devoptab layer on top of the savedata openers, reached
    // through stdio rather than through the commands directly.
    FsSaveDataInfo info;
    if (!savedata_find_account_save(&info)) {
        return TEST_SKIPPED;
    }

    //* When
    const Result rc =
        fsdevMountSaveData(SAVEDATA_MOUNT_NAME, info.application_id, info.uid);

    //* Then
    // The device was registered, and a path naming it resolves to the savedata
    // root. The listing may be empty; what is under test is that the walk gets
    // that far.
    if (R_FAILED(rc)) {
        return TEST_ASSERTION_FAILED;
    }

    DIR* dir = opendir(SAVEDATA_MOUNT_NAME ":/");
    const bool opened = dir != NULL;
    if (opened) {
        while (readdir(dir) != NULL) {
            // Walked to exhaustion so the listing is exercised rather than just
            // the open.
        }
        closedir(dir);
    }

    fsdevUnmountDevice(SAVEDATA_MOUNT_NAME);
    return opened ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
