#pragma once

#include "../../harness.h"

/**
 * @brief Test that bytes written to a file come back unchanged.
 *
 * Writes a known string through stdio and reads it back, exercising the open,
 * write, read and close path a homebrew binary actually calls.
 */
test_rc_t test_0001_write_then_read_returns_same_bytes(void);

/**
 * @brief Test that stat reports a file's size and kind, by path and by descriptor.
 *
 * Writes a file of known length and asks about it both ways, then asks about
 * the directory holding it, so both the file and directory kinds are covered.
 */
test_rc_t test_0002_stat_reports_size_and_type(void);

/**
 * @brief Test that seeking moves where the next read starts.
 *
 * Reads one byte after seeking from the start, from the current position and
 * from the end of a file whose bytes each name their own offset.
 */
test_rc_t test_0003_seek_moves_the_read_position(void);

/**
 * @brief Test that a directory walk reports every entry exactly once.
 *
 * Fills a directory with two files and a subdirectory, walks it to exhaustion,
 * and checks both the count and that the subdirectory is reported as one.
 */
test_rc_t test_0004_opendir_lists_every_entry_once(void);

/**
 * @brief Test that renaming an entry moves it and keeps its contents.
 *
 * Renames a file with known contents, then checks the old name resolves to
 * nothing, the new one resolves to the file, and the bytes came along.
 */
test_rc_t test_0005_rename_moves_the_entry(void);

/**
 * @brief Test that a file and an empty directory can be removed.
 *
 * Removes one of each, which the device does through different commands, and
 * checks neither path resolves afterwards.
 */
test_rc_t test_0006_unlink_and_rmdir_remove_entries(void);

/**
 * @brief Test that opening a path that names nothing reports why it failed.
 *
 * Checks the failure arrives as `ENOENT` rather than a generic I/O error,
 * which is what a caller branches on.
 */
test_rc_t test_0007_opening_a_missing_file_reports_enoent(void);

/**
 * Test suite for the SD card filesystem device.
 */
static void fs_sdmc_suite(void) {
    TEST_SUITE("fs/sdmc");

    TEST_CASE(
        "Test 0001: write_then_read_returns_same_bytes",
        test_0001_write_then_read_returns_same_bytes
    )
    TEST_CASE(
        "Test 0002: stat_reports_size_and_type",
        test_0002_stat_reports_size_and_type
    )
    TEST_CASE(
        "Test 0003: seek_moves_the_read_position",
        test_0003_seek_moves_the_read_position
    )
    TEST_CASE(
        "Test 0004: opendir_lists_every_entry_once",
        test_0004_opendir_lists_every_entry_once
    )
    TEST_CASE(
        "Test 0005: rename_moves_the_entry",
        test_0005_rename_moves_the_entry
    )
    TEST_CASE(
        "Test 0006: unlink_and_rmdir_remove_entries",
        test_0006_unlink_and_rmdir_remove_entries
    )
    TEST_CASE(
        "Test 0007: opening_a_missing_file_reports_enoent",
        test_0007_opening_a_missing_file_reports_enoent
    )
}
