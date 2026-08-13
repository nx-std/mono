#pragma once

#include "nx_tests_harness.h"

/**
 * @brief Test that a socket descriptor is issued and released.
 *
 * Creates a socket, closes it, and closes it again: the second close is what
 * shows the first released the table slot rather than leaving it occupied.
 */
test_rc_t test_0001_socket_create_and_close_round_trips(void);

/**
 * @brief Test that a bound socket reports the address it was bound to.
 *
 * Carries an address down to the service and back up into a caller's buffer,
 * checking the reported length as closely as the address.
 */
test_rc_t test_0002_bind_then_getsockname_reports_the_address(void);

/**
 * @brief Test that a payload survives a TCP round trip over loopback.
 *
 * Listen, connect, accept, send and receive, with both ends in this process:
 * the first test that proves bytes move rather than that descriptors exist.
 */
test_rc_t test_0003_tcp_loopback_round_trips_a_payload(void);

/**
 * @brief Test that `read` and `write` reach a socket.
 *
 * The only socket operations with no socket-specific C symbol behind them: they
 * arrive through the descriptor table, so this is the one test that fails if
 * the socket device was never registered.
 */
test_rc_t test_0004_read_and_write_reach_the_socket(void);

/**
 * @brief Test that a datagram and its sender survive a UDP round trip.
 *
 * Covers the receive that reports an address, comparing the reported sender
 * against what the sending socket says about itself.
 */
test_rc_t test_0005_udp_loopback_round_trips_a_datagram(void);

/**
 * @brief Test that a non-blocking accept with nothing queued reports `EAGAIN`.
 *
 * Exercises `fcntl` reaching the service, and the translation out of the Linux
 * error numbering the service answers in — a condition a caller polls on, so
 * the wrong number makes it spin forever.
 */
test_rc_t test_0006_nonblocking_accept_on_an_idle_listener_reports_eagain(void);

/**
 * @brief Test that `poll` reports a socket readable once its peer sends.
 *
 * Polls before and after the send: an implementation that lost the descriptor
 * correspondence could report readable, but not not-readable first.
 */
test_rc_t test_0007_poll_reports_readability_when_data_arrives(void);

/**
 * @brief Test that `select` reports a socket readable once its peer sends.
 *
 * `select` is answered by a `poll` underneath, because its bitmaps cannot
 * survive renumbering; this checks the set coming back names the descriptor the
 * caller asked about.
 */
test_rc_t test_0008_select_reports_readability_when_data_arrives(void);

/**
 * @brief Test that socket calls refuse a descriptor that names no socket.
 *
 * Distinguishes `ENOTSOCK`, for a descriptor backed by something else, from
 * `EBADF`, for one nothing opened.
 */
test_rc_t test_0009_socket_calls_on_a_non_socket_report_enotsock(void);

/**
 * @brief Test that a socket option survives a write and a read back.
 *
 * The option's bytes and its length travel both ways, and the length is written
 * into the caller's own variable.
 */
test_rc_t test_0010_sockopt_round_trips_a_value(void);

/**
 * @brief Test that each end of a connection reports the other.
 *
 * The two answers have to cross, which an implementation reporting the socket's
 * own address for both would fail.
 */
test_rc_t test_0011_getpeername_reports_the_connected_peer(void);

/**
 * @brief Test that a shutdown ends the peer's receive with a zero count.
 *
 * The end of a stream is a zero byte count rather than a failure; reporting it
 * as an error would make every correct reader treat a normal close as a fault.
 */
test_rc_t test_0012_shutdown_ends_the_peer_receive(void);

/**
 * Test suite for the BSD socket surface.
 */
static void net_socket_suite(void) {
    TEST_SUITE("net/socket");

    TEST_CASE(
        "Test 0001: socket_create_and_close_round_trips",
        test_0001_socket_create_and_close_round_trips
    )
    TEST_CASE(
        "Test 0002: bind_then_getsockname_reports_the_address",
        test_0002_bind_then_getsockname_reports_the_address
    )
    TEST_CASE(
        "Test 0003: tcp_loopback_round_trips_a_payload",
        test_0003_tcp_loopback_round_trips_a_payload
    )
    TEST_CASE(
        "Test 0004: read_and_write_reach_the_socket",
        test_0004_read_and_write_reach_the_socket
    )
    TEST_CASE(
        "Test 0005: udp_loopback_round_trips_a_datagram",
        test_0005_udp_loopback_round_trips_a_datagram
    )
    TEST_CASE(
        "Test 0006: nonblocking_accept_on_an_idle_listener_reports_eagain",
        test_0006_nonblocking_accept_on_an_idle_listener_reports_eagain
    )
    TEST_CASE(
        "Test 0007: poll_reports_readability_when_data_arrives",
        test_0007_poll_reports_readability_when_data_arrives
    )
    TEST_CASE(
        "Test 0008: select_reports_readability_when_data_arrives",
        test_0008_select_reports_readability_when_data_arrives
    )
    TEST_CASE(
        "Test 0009: socket_calls_on_a_non_socket_report_enotsock",
        test_0009_socket_calls_on_a_non_socket_report_enotsock
    )
    TEST_CASE(
        "Test 0010: sockopt_round_trips_a_value",
        test_0010_sockopt_round_trips_a_value
    )
    TEST_CASE(
        "Test 0011: getpeername_reports_the_connected_peer",
        test_0011_getpeername_reports_the_connected_peer
    )
    TEST_CASE(
        "Test 0012: shutdown_ends_the_peer_receive",
        test_0012_shutdown_ends_the_peer_receive
    )
}
