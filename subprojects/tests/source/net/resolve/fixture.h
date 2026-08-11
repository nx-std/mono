#pragma once

#include <arpa/inet.h>
#include <netdb.h>
#include <netinet/in.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>
#include <sys/socket.h>

/**
 * @brief A name that has to be looked up over the network to resolve.
 *
 * Only the one test that deliberately needs the console online uses it. Every
 * other test here resolves a literal, which the resolver answers without
 * leaving the console.
 */
#define RESOLVE_PUBLIC_NAME "example.com"

/**
 * @brief The loopback literal the offline tests resolve.
 *
 * A numeric host is still a full round trip through the resolver service — the
 * resolver does not shortcut it locally — so a test using one exercises the
 * whole path, including the service-manager session the session is acquired
 * over, without depending on a network being reachable.
 */
#define RESOLVE_NUMERIC_HOST "127.0.0.1"

/**
 * @brief The numeric service the offline tests pair with the host above.
 */
#define RESOLVE_NUMERIC_SERVICE "80"

/** @brief The port `RESOLVE_NUMERIC_SERVICE` names. */
#define RESOLVE_NUMERIC_PORT 80

/**
 * @brief Whether `list` holds an IPv4 entry for the loopback address.
 *
 * Walks the whole chain rather than checking the first entry: the resolver may
 * report several records for one name, and which comes first is its business.
 */
static inline bool resolve_list_has_loopback(const struct addrinfo* list, uint16_t port) {
    for (const struct addrinfo* it = list; it != NULL; it = it->ai_next) {
        if (it->ai_family != AF_INET || it->ai_addr == NULL) {
            continue;
        }
        if (it->ai_addrlen < sizeof(struct sockaddr_in)) {
            continue;
        }

        const struct sockaddr_in* addr = (const struct sockaddr_in*)it->ai_addr;
        if (addr->sin_family == AF_INET
            && ntohl(addr->sin_addr.s_addr) == INADDR_LOOPBACK
            && ntohs(addr->sin_port) == port) {
            return true;
        }
    }
    return false;
}
