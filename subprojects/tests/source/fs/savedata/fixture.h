#pragma once

#include <string.h>

#include <switch.h>

/**
 * @brief Name the savedata mounts in this suite are registered under.
 *
 * Named after the suite so a run that dies partway leaves an identifiable
 * device behind rather than one that looks like the program's own.
 */
#define SAVEDATA_MOUNT_NAME "nxtestsave"

/**
 * @brief An application id no title owns.
 *
 * The top of the range is reserved, so no savedata can be keyed by it. Used to
 * check that a savedata that does not exist is reported as an error rather than
 * opened.
 */
#define SAVEDATA_UNKNOWN_APPLICATION_ID ((u64)0xFFFFFFFFFFFFFFFFULL)

/**
 * @brief The first account savedata the console holds, if it holds any.
 *
 * Several tests need a savedata that actually exists, and which one that is
 * depends on the console. Asking the info reader is the only way to find out,
 * which is why this walk is shared rather than repeated: a test that hard-coded
 * an application id would pass on one console and fail on the next.
 *
 * `FsSaveDataSpaceId_User` holds kinds other than `FsSaveDataType_Account`, so
 * the walk filters rather than taking the first entry.
 *
 * @return true when one was found and `out_info` was filled.
 */
static inline bool savedata_find_account_save(FsSaveDataInfo* out_info) {
    FsSaveDataInfoReader reader;
    if (R_FAILED(fsOpenSaveDataInfoReader(&reader, FsSaveDataSpaceId_User))) {
        return false;
    }

    bool found = false;
    while (!found) {
        FsSaveDataInfo info;
        s64 entries = 0;
        if (R_FAILED(fsSaveDataInfoReaderRead(&reader, &info, 1, &entries))
            || entries == 0) {
            break;
        }

        if (info.save_data_type == FsSaveDataType_Account) {
            *out_info = info;
            found = true;
        }
    }

    fsSaveDataInfoReaderClose(&reader);
    return found;
}

/**
 * @brief Counts the entries a reader still has, closing it.
 *
 * Walks to exhaustion one entry at a time, which is what a caller that does not
 * know the total does, and is the shape the `save` example uses.
 *
 * @return the number of entries read, or -1 when a read failed.
 */
static inline s64 savedata_drain_reader(FsSaveDataInfoReader* reader) {
    s64 count = 0;
    while (true) {
        FsSaveDataInfo info;
        s64 entries = 0;
        const Result rc = fsSaveDataInfoReaderRead(reader, &info, 1, &entries);
        if (R_FAILED(rc)) {
            return -1;
        }
        if (entries == 0) {
            return count;
        }
        // A read that reports more entries than the buffer holds wrote past it.
        if (entries > 1) {
            return -1;
        }
        count += entries;
    }
}
