#include <string.h>

#include <switch.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0004: open save data info reader with filter reports only account saves">

test_rc_t test_0004_open_save_data_info_reader_with_filter_reports_only_account_saves(void)
{
    //* Given
    // A filter admitting only account savedata. The command carrying it arrived
    // in HOS 6.0.0, so an older console has nothing to answer with.
    if (hosversionBefore(6, 0, 0)) {
        return TEST_SKIPPED;
    }

    FsSaveDataFilter filter;
    memset(&filter, 0, sizeof(filter));
    filter.filter_by_save_data_type = true;
    filter.attr.save_data_type = FsSaveDataType_Account;

    //* When
    FsSaveDataInfoReader reader;
    const Result rc = fsOpenSaveDataInfoReaderWithFilter(
        &reader, FsSaveDataSpaceId_User, &filter);

    //* Then
    // The reader opened, and everything it reports is the kind asked for. An
    // empty result is admissible; a result of the wrong kind is not, and would
    // mean the filter never reached the server.
    if (R_FAILED(rc)) {
        return TEST_ASSERTION_FAILED;
    }

    bool correct = true;
    while (correct) {
        FsSaveDataInfo info;
        s64 entries = 0;
        if (R_FAILED(fsSaveDataInfoReaderRead(&reader, &info, 1, &entries))) {
            correct = false;
            break;
        }
        if (entries == 0) {
            break;
        }
        if (info.save_data_type != FsSaveDataType_Account) {
            correct = false;
        }
    }

    fsSaveDataInfoReaderClose(&reader);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
