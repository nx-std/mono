#pragma once

#include "nx_tests_harness.h"

/**
 * @brief Test that a literal address resolves to itself.
 *
 * A numeric host is still a full round trip through the resolver service — it
 * is not shortcut locally — so this covers the whole path, including the
 * service-manager session the resolver session is acquired over, without
 * needing the console to be on a network.
 */
test_rc_t test_0001_getaddrinfo_with_a_numeric_host_returns_that_address(void);

/**
 * @brief Test that a lookup naming nothing is refused.
 *
 * With neither a node nor a service there is nothing to resolve, so the
 * refusal comes from the argument check; the caller's result slot must be left
 * empty, since a chain reported beside a failure is one nobody frees.
 */
test_rc_t test_0002_getaddrinfo_with_no_host_and_no_service_fails(void);

/**
 * @brief Test that an address resolves to a host and service description.
 *
 * The reverse direction, over the same session. It asks for numeric output but
 * does not require it: the service answers with names regardless, and libnx
 * forwards the same flags and gets the same answer, so that is the platform's
 * behaviour rather than this implementation's.
 */
test_rc_t test_0003_getnameinfo_describes_an_address(void);

/**
 * @brief Test that every resolver code has a description.
 *
 * A table lookup that reaches no service, so it holds whether or not a session
 * was ever established — including for a code the resolver never reports,
 * which a caller still prints without checking.
 */
test_rc_t test_0004_gai_strerror_describes_every_code(void);

/**
 * @brief Test that the older resolver entry point answers for a literal.
 *
 * Covers a second of the five symbols that need the service-manager session,
 * and the `hostent` block it answers with rather than an `addrinfo` chain.
 */
test_rc_t test_0005_gethostbyname_with_a_numeric_host_returns_that_address(void);

/**
 * @brief Test that a public name resolves over the network.
 *
 * The only test here that needs the console online. A failure is reported as a
 * skip, because an offline console and a broken lookup are indistinguishable
 * from inside the process — so this confirms a real lookup when it can, and
 * accuses nothing when it cannot.
 */
test_rc_t test_0006_getaddrinfo_resolves_a_public_name(void);

/**
 * Test suite for name resolution.
 */
static void net_resolve_suite(void) {
    TEST_SUITE("net/resolve");

    TEST_CASE(
        "Test 0001: getaddrinfo_with_a_numeric_host_returns_that_address",
        test_0001_getaddrinfo_with_a_numeric_host_returns_that_address
    )
    TEST_CASE(
        "Test 0002: getaddrinfo_with_no_host_and_no_service_fails",
        test_0002_getaddrinfo_with_no_host_and_no_service_fails
    )
    TEST_CASE(
        "Test 0003: getnameinfo_describes_an_address",
        test_0003_getnameinfo_describes_an_address
    )
    TEST_CASE(
        "Test 0004: gai_strerror_describes_every_code",
        test_0004_gai_strerror_describes_every_code
    )
    TEST_CASE(
        "Test 0005: gethostbyname_with_a_numeric_host_returns_that_address",
        test_0005_gethostbyname_with_a_numeric_host_returns_that_address
    )
    TEST_CASE(
        "Test 0006: getaddrinfo_resolves_a_public_name",
        test_0006_getaddrinfo_resolves_a_public_name
    )
}
