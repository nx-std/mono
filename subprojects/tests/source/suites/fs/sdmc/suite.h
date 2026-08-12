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
 * @brief Test that a relative path is resolved against the working directory.
 *
 * Moves into the fixture directory and writes a file by a name carrying neither
 * device nor directory, which only reaches the right place if the device joins
 * it onto the directory it was told to work in.
 */
test_rc_t test_0008_chdir_makes_relative_paths_resolve(void);

/**
 * @brief Test that the working directory a device accepted is the one reported.
 *
 * Changes directory and asks for it back, which fails outright if the device
 * refuses a directory it should have taken.
 */
test_rc_t test_0009_getcwd_reports_the_working_directory(void);

/**
 * @brief Test that creating a directory twice reports why the second failed.
 *
 * Checks the failure arrives as `EEXIST` rather than a generic I/O error, which
 * is what a caller creating a directory it may already own branches on.
 */
test_rc_t test_0010_mkdir_on_an_existing_entry_reports_eexist(void);

/**
 * @brief Test that a directory holding entries cannot be removed.
 *
 * Checks the removal is refused and that neither the directory nor what it
 * holds went anywhere.
 */
test_rc_t test_0011_rmdir_on_a_non_empty_directory_fails(void);

/**
 * @brief Test that an appending descriptor writes at the end of the file.
 *
 * Seeks to the start before writing, which an appending descriptor must ignore,
 * so a device that writes at the position rather than the end fails here.
 */
test_rc_t test_0012_append_mode_writes_at_the_end(void);

/**
 * @brief Test that cutting a file short changes the size it reports.
 *
 * Truncates through the descriptor and asks about the path, so the resize and
 * the size query have to agree with each other.
 */
test_rc_t test_0013_ftruncate_shortens_the_file(void);

/**
 * @brief Test that a payload too large for one request survives the trip.
 *
 * Writes and reads back 64 KiB whose every byte depends on its offset, which a
 * transfer that dropped or reordered a stretch cannot reproduce.
 */
test_rc_t test_0014_write_larger_than_one_command_round_trips(void);

/**
 * @brief Test that two descriptors on one file keep separate positions.
 *
 * Reads through both and checks neither moved the other, which is what breaks
 * if the two descriptors end up sharing one open file.
 */
test_rc_t test_0015_two_descriptors_on_one_file_are_independent(void);

/**
 * @brief Test that the card reports a capacity and how much of it is free.
 *
 * Checks the figures are in bytes and consistent with each other, rather than
 * the zeroes a device that does not answer the query would leave.
 */
test_rc_t test_0016_statvfs_reports_the_card_capacity(void);

/**
 * @brief Test that an exclusive create refuses a path that is already taken.
 *
 * Checks the refusal arrives as `EEXIST` rather than as the missing-path error
 * the open would report if the create's failure were dropped.
 */
test_rc_t test_0017_exclusive_create_on_an_existing_file_reports_eexist(void);

/**
 * @brief Test that a duplicated descriptor names the same open file.
 *
 * Writes through both descriptors and checks the second carried on from where
 * the first stopped, which only holds if they share one position rather than
 * each holding a file of its own.
 */
test_rc_t test_0018_dup_gives_a_second_descriptor_onto_one_file(void);

/**
 * @brief Test that closing one of two descriptors on a file leaves it open.
 *
 * Writes through the survivor after the other is closed, which fails if the
 * first close told the device to close the file instead of counting the
 * descriptors still naming it.
 */
test_rc_t test_0019_closing_one_duplicate_leaves_the_file_open(void);

/**
 * @brief Test that a descriptor rebound onto another file reaches that file.
 *
 * Checks both halves of the exchange: the write through the rebound number
 * lands in the file it now names, and the file it stopped naming was closed
 * with what it had.
 */
test_rc_t test_0020_dup2_rebinds_a_descriptor_onto_another_file(void);

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
    TEST_CASE(
        "Test 0008: chdir_makes_relative_paths_resolve",
        test_0008_chdir_makes_relative_paths_resolve
    )
    TEST_CASE(
        "Test 0009: getcwd_reports_the_working_directory",
        test_0009_getcwd_reports_the_working_directory
    )
    TEST_CASE(
        "Test 0010: mkdir_on_an_existing_entry_reports_eexist",
        test_0010_mkdir_on_an_existing_entry_reports_eexist
    )
    TEST_CASE(
        "Test 0011: rmdir_on_a_non_empty_directory_fails",
        test_0011_rmdir_on_a_non_empty_directory_fails
    )
    TEST_CASE(
        "Test 0012: append_mode_writes_at_the_end",
        test_0012_append_mode_writes_at_the_end
    )
    TEST_CASE(
        "Test 0013: ftruncate_shortens_the_file",
        test_0013_ftruncate_shortens_the_file
    )
    TEST_CASE(
        "Test 0014: write_larger_than_one_command_round_trips",
        test_0014_write_larger_than_one_command_round_trips
    )
    TEST_CASE(
        "Test 0015: two_descriptors_on_one_file_are_independent",
        test_0015_two_descriptors_on_one_file_are_independent
    )
    TEST_CASE(
        "Test 0016: statvfs_reports_the_card_capacity",
        test_0016_statvfs_reports_the_card_capacity
    )
    TEST_CASE(
        "Test 0017: exclusive_create_on_an_existing_file_reports_eexist",
        test_0017_exclusive_create_on_an_existing_file_reports_eexist
    )
    TEST_CASE(
        "Test 0018: dup_gives_a_second_descriptor_onto_one_file",
        test_0018_dup_gives_a_second_descriptor_onto_one_file
    )
    TEST_CASE(
        "Test 0019: closing_one_duplicate_leaves_the_file_open",
        test_0019_closing_one_duplicate_leaves_the_file_open
    )
    TEST_CASE(
        "Test 0020: dup2_rebinds_a_descriptor_onto_another_file",
        test_0020_dup2_rebinds_a_descriptor_onto_another_file
    )
}
