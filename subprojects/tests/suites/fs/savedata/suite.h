#pragma once

#include "nx_tests_harness.h"

/**
 * @brief Test that a reader over the user savedata space can be opened.
 *
 * The first command the `save` example reaches, and the one whose absence used
 * to hang the process rather than fail it.
 */
test_rc_t test_0001_open_save_data_info_reader_over_the_user_space_succeeds(void);

/**
 * @brief Test that walking a reader one entry at a time terminates.
 *
 * Reads until the reader reports nothing left, which is the shape a caller that
 * does not know the total uses. A console with no user savedata still passes:
 * what is checked is that the walk ends and stays inside its buffer.
 */
test_rc_t test_0002_save_data_info_reader_read_walks_to_exhaustion(void);

/**
 * @brief Test that a reader over every space at once can be opened.
 *
 * `FsSaveDataSpaceId_All` selects a different command from a real space id, so
 * this covers the half of that branch test 0001 does not.
 */
test_rc_t test_0003_open_save_data_info_reader_over_every_space_succeeds(void);

/**
 * @brief Test that a filtered reader reports only the savedata kind asked for.
 *
 * Skipped before HOS 6.0.0, where the command does not exist. An entry of
 * another kind means the filter never reached the server.
 */
test_rc_t test_0004_open_save_data_info_reader_with_filter_reports_only_account_saves(void);

/**
 * @brief Test that an account savedata found through the reader can be opened.
 *
 * Opens the root directory as well, so the object handed back is one the server
 * takes commands on rather than just a non-zero id. Skipped on a console
 * holding no account savedata.
 */
test_rc_t test_0005_open_save_data_opens_an_account_save(void);

/**
 * @brief Test that the read-only opener reaches the same savedata.
 *
 * A separate command from the writable opener, with a HOS 2.0.0 floor of its
 * own.
 */
test_rc_t test_0006_open_save_data_read_only_opens_the_same_save(void);

/**
 * @brief Test that opening a savedata no title owns is refused.
 *
 * Returning at all is half of what is checked: the failure this suite exists
 * for was a command that never came back.
 */
test_rc_t test_0007_open_save_data_for_an_unknown_application_fails(void);

/**
 * @brief Test that a mounted savedata resolves as a device.
 *
 * The `save` example's path end to end: the devoptab layer on top of the
 * openers, reached through stdio. Skipped on a console holding no account
 * savedata.
 */
test_rc_t test_0008_fsdev_mount_save_data_lists_the_save_root(void);

/**
 * Test suite for savedata openers and the savedata info reader.
 */
static void fs_savedata_suite(void) {
    TEST_SUITE("fs/savedata");

    TEST_CASE(
        "Test 0001: open_save_data_info_reader_over_the_user_space_succeeds",
        test_0001_open_save_data_info_reader_over_the_user_space_succeeds
    )
    TEST_CASE(
        "Test 0002: save_data_info_reader_read_walks_to_exhaustion",
        test_0002_save_data_info_reader_read_walks_to_exhaustion
    )
    TEST_CASE(
        "Test 0003: open_save_data_info_reader_over_every_space_succeeds",
        test_0003_open_save_data_info_reader_over_every_space_succeeds
    )
    TEST_CASE(
        "Test 0004: open_save_data_info_reader_with_filter_reports_only_account_saves",
        test_0004_open_save_data_info_reader_with_filter_reports_only_account_saves
    )
    TEST_CASE(
        "Test 0005: open_save_data_opens_an_account_save",
        test_0005_open_save_data_opens_an_account_save
    )
    TEST_CASE(
        "Test 0006: open_save_data_read_only_opens_the_same_save",
        test_0006_open_save_data_read_only_opens_the_same_save
    )
    TEST_CASE(
        "Test 0007: open_save_data_for_an_unknown_application_fails",
        test_0007_open_save_data_for_an_unknown_application_fails
    )
    TEST_CASE(
        "Test 0008: fsdev_mount_save_data_lists_the_save_root",
        test_0008_fsdev_mount_save_data_lists_the_save_root
    )
}
