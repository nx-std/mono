#pragma once

#include "nx_tests_harness.h"

/**
 * @brief Test that a program can mount the image bundled inside itself.
 *
 * Exercises the whole of `romfsMountSelf` for an NRO: the command line names
 * the file, the asset header inside it says where the image starts, and the
 * device registers under the name.
 */
test_rc_t test_0001_romfs_init_mounts_the_programs_own_image(void);

/**
 * @brief Test that reading a file returns the bytes it was built with.
 *
 * Reads a small file through stdio, exercising the open, read and close path a
 * homebrew binary actually calls.
 */
test_rc_t test_0002_reading_a_file_returns_the_bundled_bytes(void);

/**
 * @brief Test that stat reports a file's size and kind, by path and by descriptor.
 *
 * Asks about a file of known length both ways, then about a directory, so both
 * kinds are covered.
 */
test_rc_t test_0003_stat_reports_size_and_type(void);

/**
 * @brief Test that a directory walk reports every entry exactly once.
 *
 * Walks a directory holding one file and one subdirectory to exhaustion, and
 * pins the two synthetic entries a romfs walk produces where the filesystem
 * device produces none.
 */
test_rc_t test_0004_opendir_lists_every_entry_once(void);

/**
 * @brief Test that seeking moves where the next read starts.
 *
 * Reaches one line of a file three ways — from the start, from the current
 * position and backwards from the end — and checks all three landed on it.
 */
test_rc_t test_0005_seek_moves_the_read_position(void);

/**
 * @brief Test that a file too large for one read survives the trip.
 *
 * Reads 16 KiB whose every line names its own index, which a transfer that
 * dropped or repeated a stretch cannot reproduce even though it would still
 * fill the buffer.
 */
test_rc_t test_0006_a_file_larger_than_one_read_round_trips(void);

/**
 * @brief Test that opening a path that names nothing reports why it failed.
 *
 * Checks the failure arrives as `ENOENT` rather than a generic I/O error,
 * which is what a caller branches on.
 */
test_rc_t test_0007_opening_a_missing_file_reports_enoent(void);

/**
 * @brief Test that an image refuses to be written to.
 *
 * Opens an existing file for writing and a missing one for creation, and checks
 * the file that was there is unchanged afterwards.
 */
test_rc_t test_0008_opening_for_writing_is_refused(void);

/**
 * @brief Test that a relative path is resolved against the working directory.
 *
 * Moves into a directory of the image and opens a file by a name carrying
 * neither device nor directory.
 */
test_rc_t test_0009_chdir_makes_relative_paths_resolve(void);

/**
 * @brief Test that a name that is not ASCII resolves to its file.
 *
 * An image stores names as bytes and a lookup compares them as bytes; this
 * fails if anything on the route decodes them instead.
 */
test_rc_t test_0010_a_non_ascii_name_resolves(void);

/**
 * @brief Test that unmounting stops the paths resolving, and mounting works again.
 *
 * Covers both halves: the name stops resolving, and the device it left behind
 * can be filled a second time.
 */
test_rc_t test_0011_unmounting_makes_the_paths_stop_resolving(void);

/**
 * @brief Test that mounting an already-mounted name is refused.
 *
 * Checks the second mount fails and the first is untouched, rather than being
 * silently replaced under whatever already had it open.
 */
test_rc_t test_0012_mounting_a_name_twice_is_refused(void);

/**
 * Test suite for the read-only filesystem device.
 */
static void romfs_suite(void) {
    TEST_SUITE("romfs");

    TEST_CASE(
        "Test 0001: romfs_init_mounts_the_programs_own_image",
        test_0001_romfs_init_mounts_the_programs_own_image
    )
    TEST_CASE(
        "Test 0002: reading_a_file_returns_the_bundled_bytes",
        test_0002_reading_a_file_returns_the_bundled_bytes
    )
    TEST_CASE(
        "Test 0003: stat_reports_size_and_type",
        test_0003_stat_reports_size_and_type
    )
    TEST_CASE(
        "Test 0004: opendir_lists_every_entry_once",
        test_0004_opendir_lists_every_entry_once
    )
    TEST_CASE(
        "Test 0005: seek_moves_the_read_position",
        test_0005_seek_moves_the_read_position
    )
    TEST_CASE(
        "Test 0006: a_file_larger_than_one_read_round_trips",
        test_0006_a_file_larger_than_one_read_round_trips
    )
    TEST_CASE(
        "Test 0007: opening_a_missing_file_reports_enoent",
        test_0007_opening_a_missing_file_reports_enoent
    )
    TEST_CASE(
        "Test 0008: opening_for_writing_is_refused",
        test_0008_opening_for_writing_is_refused
    )
    TEST_CASE(
        "Test 0009: chdir_makes_relative_paths_resolve",
        test_0009_chdir_makes_relative_paths_resolve
    )
    TEST_CASE(
        "Test 0010: a_non_ascii_name_resolves",
        test_0010_a_non_ascii_name_resolves
    )
    TEST_CASE(
        "Test 0011: unmounting_makes_the_paths_stop_resolving",
        test_0011_unmounting_makes_the_paths_stop_resolving
    )
    TEST_CASE(
        "Test 0012: mounting_a_name_twice_is_refused",
        test_0012_mounting_a_name_twice_is_refused
    )
}
