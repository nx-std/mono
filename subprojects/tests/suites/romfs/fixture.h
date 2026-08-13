#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

#include <switch.h>

/**
 * @brief What the image bundled with this binary holds.
 *
 * The tree under `assets/` is packed into the NRO at build time and mounted at
 * run time, so these names and these bytes are the same fixture on every
 * console. Nothing here is written, ever: an image cannot be, which is what
 * lets every test share one copy without ordering between them.
 */
#define ROMFS_HELLO_PATH "romfs:/hello.txt"
#define ROMFS_HELLO_CONTENT "Hello from romfs!\n"

/** A directory holding both a file and a subdirectory. */
#define ROMFS_DATA_DIR "romfs:/data"

/** A file large enough that reading it spans several commands. */
#define ROMFS_LINES_PATH "romfs:/data/lines.txt"

/** Every line of it is this many bytes, newline included. */
#define ROMFS_LINES_LINE_LEN 32

/** And there are this many, so the file is 16 KiB exactly. */
#define ROMFS_LINES_COUNT 512

/** A file one directory further down, for resolving a path in two steps. */
#define ROMFS_LEAF_PATH "romfs:/data/nested/leaf.txt"
#define ROMFS_LEAF_CONTENT "leaf\n"

/** A name that is not ASCII, which the lookup compares as bytes. */
#define ROMFS_NON_ASCII_PATH "romfs:/フォルダ/ファイル.txt"
#define ROMFS_NON_ASCII_CONTENT "non-ascii\n"

/**
 * @brief Mounts this binary's own image under `romfs:`.
 *
 * Each test mounts and unmounts around itself rather than sharing one mount for
 * the run. A test that shared it would pass or fail depending on what the test
 * before it left mounted, and the mount is the thing under test here.
 *
 * @return true when the image was mounted.
 */
static inline bool romfs_fixture_open(void) {
    return R_SUCCEEDED(romfsInit());
}

/**
 * @brief Gives the image back.
 *
 * Called once per test, after its verdict has been decided, so nothing it
 * asserts on can be disturbed by the unmount.
 */
static inline void romfs_fixture_close(void) {
    romfsExit();
}

/**
 * @brief Reads `path` into `buf` as a nul-terminated string.
 *
 * @return true when the file was read and fit in `buf` with room for the nul.
 */
static inline bool romfs_read_file(const char* path, char* buf, size_t buf_len) {
    FILE* file = fopen(path, "r");
    if (file == NULL) {
        return false;
    }

    const size_t read = fread(buf, 1, buf_len - 1, file);
    buf[read] = '\0';

    return fclose(file) == 0;
}

/**
 * @brief Builds the line the large file holds at index `index`.
 *
 * The file is generated rather than written by hand, so what it should contain
 * is stated here as the same rule that generated it.
 */
static inline void romfs_expected_line(size_t index, char out[ROMFS_LINES_LINE_LEN + 1]) {
    // The trailing space before the newline is the padding that makes every
    // line the same length; without it the offsets a seek test computes from
    // the line number would all be wrong by the same amount and still agree.
    snprintf(out, ROMFS_LINES_LINE_LEN + 1, "line %04zu: the quick brown fox \n", index);
}
