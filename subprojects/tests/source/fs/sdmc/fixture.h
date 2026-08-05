#pragma once

#include <dirent.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

/**
 * @brief Directory the suite works inside.
 *
 * Named after the binary so a run that dies partway leaves something
 * identifiable on the SD card rather than litter under the root.
 */
#define SDMC_ROOT "sdmc:/nx-tests-fs"

/**
 * @brief Builds the path of a directory belonging to one test.
 *
 * Every test works inside its own, named after itself. Sharing one directory
 * would make each test's setup responsible for cleaning up after whichever test
 * ran before it: a leak would be silently absorbed by the next reset, and a
 * failure would be attributable to no one in particular.
 */
#define SDMC_TEST_DIR(test_name) SDMC_ROOT "/" test_name

/**
 * @brief Removes `path` and everything under it, ignoring what was not there.
 *
 * One level of nesting is enough — no test here builds deeper.
 */
static inline void sdmc_remove_tree(const char* path) {
    DIR* dir = opendir(path);
    if (dir != NULL) {
        struct dirent* ent;
        while ((ent = readdir(dir)) != NULL) {
            // Skip anything whose full path does not fit rather than acting on
            // a truncated one, which would name a different entry.
            char child[512];
            const int len =
                snprintf(child, sizeof(child), "%s/%s", path, ent->d_name);
            if (len < 0 || (size_t)len >= sizeof(child)) {
                continue;
            }

            if (ent->d_type == DT_DIR) {
                sdmc_remove_tree(child);
            } else {
                unlink(child);
            }
        }
        closedir(dir);
    }
    rmdir(path);
}

/**
 * @brief Gives a test an empty directory of its own to work in.
 *
 * Removes whatever a previous run left at `dir` before recreating it, so a test
 * starts from the same state whether or not the last run finished.
 *
 * @return true when the directory exists and is empty afterwards.
 */
static inline bool sdmc_fixture_open(const char* dir) {
    // The suite's own directory may or may not be there; only its absence after
    // this call is a problem, which the per-test `mkdir` below reports.
    mkdir(SDMC_ROOT, 0777);

    sdmc_remove_tree(dir);
    return mkdir(dir, 0777) == 0;
}

/**
 * @brief Gives back the directory a test was working in.
 *
 * Called once per test, after its verdict has been decided, so that nothing it
 * asserts on can be disturbed by the cleanup — an `errno` read after this would
 * report whatever the removal did, not what the test saw.
 */
static inline void sdmc_fixture_close(const char* dir) {
    sdmc_remove_tree(dir);
}

/**
 * @brief Writes `content` to `path`, replacing whatever was there.
 *
 * @return true when every byte was written and the file closed cleanly.
 */
static inline bool sdmc_write_file(const char* path, const char* content) {
    FILE* file = fopen(path, "w");
    if (file == NULL) {
        return false;
    }

    const size_t len = strlen(content);
    const size_t written = fwrite(content, 1, len, file);

    return fclose(file) == 0 && written == len;
}

/**
 * @brief Reads `path` into `buf` as a nul-terminated string.
 *
 * @return true when the file was read and fit in `buf` with room for the nul.
 */
static inline bool sdmc_read_file(const char* path, char* buf, size_t buf_len) {
    FILE* file = fopen(path, "r");
    if (file == NULL) {
        return false;
    }

    const size_t read = fread(buf, 1, buf_len - 1, file);
    buf[read] = '\0';

    return fclose(file) == 0;
}
