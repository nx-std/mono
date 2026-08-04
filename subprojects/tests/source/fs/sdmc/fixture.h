#pragma once

#include <dirent.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

/**
 * @brief Directory every test in this suite works inside.
 *
 * Named after the binary so a run that dies partway leaves something
 * identifiable on the SD card rather than litter under the root.
 */
#define SDMC_ROOT "sdmc:/nx-tests-fs"

/**
 * @brief Builds a path inside the suite's directory.
 */
#define SDMC_PATH(name) SDMC_ROOT "/" name

/**
 * @brief Removes `path` and everything under it, ignoring what was not there.
 *
 * Tests call this before and after their own work: before, because a previous
 * run may have died holding the fixture, and after, so the SD card is left as
 * it was found. One level of nesting is enough — no test here builds deeper.
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
 * @brief Empties the suite's directory and recreates it.
 *
 * @return true when the directory exists and is empty afterwards.
 */
static inline bool sdmc_fixture_reset(void) {
    sdmc_remove_tree(SDMC_ROOT);
    return mkdir(SDMC_ROOT, 0777) == 0;
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
