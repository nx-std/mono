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
 * @brief Test that a non-blocking connect over loopback finishes in the call.
 *
 * Pins where the answer arrives, which over loopback is the connect itself:
 * both ends are this console, so there is no in-flight state to be told about
 * later and no `EINPROGRESS` to wait out.
 */
test_rc_t test_0013_nonblocking_connect_over_loopback_completes_inline(void);

/**
 * @brief Test that a wait reports a connected socket writable.
 *
 * The half of readiness every other case here leaves untested. A caller that
 * can only be told about readable sockets has to send blind.
 */
test_rc_t test_0014_poll_reports_a_connected_socket_writable(void);

/**
 * @brief Test that a refused connect over loopback says so in the call.
 *
 * The failure half of test 0013, and the same answer: the refusal comes back
 * from the connect rather than waiting on the socket to be collected.
 */
test_rc_t test_0015_a_refused_connect_over_loopback_reports_econnrefused(void);

/**
 * @brief Test that a blocked `poll` returns when another thread sends.
 *
 * Times the wait rather than only checking what it reported: a wait that cannot
 * be ended before its timeout is one an event loop cannot be woken out of.
 */
test_rc_t test_0016_a_blocked_poll_returns_when_another_thread_sends(void);

/**
 * @brief Test that one wait can be asked about a set of many sockets.
 *
 * Every other case here polls one socket. A caller watching a connection table
 * passes the whole set on every wait, so the size it will accept is a limit on
 * what can be built over it.
 */
test_rc_t test_0017_poll_accepts_a_set_of_many_sockets(void);

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
    TEST_CASE(
        "Test 0013: nonblocking_connect_over_loopback_completes_inline",
        test_0013_nonblocking_connect_over_loopback_completes_inline
    )
    TEST_CASE(
        "Test 0014: poll_reports_a_connected_socket_writable",
        test_0014_poll_reports_a_connected_socket_writable
    )
    TEST_CASE(
        "Test 0015: a_refused_connect_over_loopback_reports_econnrefused",
        test_0015_a_refused_connect_over_loopback_reports_econnrefused
    )
    TEST_CASE(
        "Test 0016: a_blocked_poll_returns_when_another_thread_sends",
        test_0016_a_blocked_poll_returns_when_another_thread_sends
    )
    TEST_CASE(
        "Test 0017: poll_accepts_a_set_of_many_sockets",
        test_0017_poll_accepts_a_set_of_many_sockets
    )
}
