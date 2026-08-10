#pragma once

#include <arpa/inet.h>
#include <errno.h>
#include <netinet/in.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

#include <switch.h>

/**
 * @brief Port the suite binds when it needs a specific one.
 *
 * High enough to be outside anything the system claims, and fixed rather than
 * ephemeral so a run that dies partway is identifiable in a packet capture.
 * Every test that uses it binds with `SO_REUSEADDR`, so a socket left in
 * `TIME_WAIT` by the previous test does not fail the next one.
 */
#define NET_TEST_PORT 45678

/**
 * @brief Fills `addr` with the loopback address and `port`.
 *
 * Loopback is what makes this suite self-contained: both ends of every
 * connection live in this process, so nothing depends on the console being on a
 * network, on a peer being reachable, or on which address the console was
 * assigned.
 */
static inline void net_loopback_addr(struct sockaddr_in* addr, uint16_t port) {
    memset(addr, 0, sizeof(*addr));
    addr->sin_family = AF_INET;
    addr->sin_port = htons(port);
    addr->sin_addr.s_addr = htonl(INADDR_LOOPBACK);
}

/**
 * @brief Closes a descriptor if it is one, and reports -1 back.
 *
 * Every test closes on the way out of both the success and the failure path, so
 * a failing test does not leak a socket into the next one. The service has a
 * finite descriptor table per client, and a leak would surface as an unrelated
 * test failing to create a socket.
 */
static inline int net_close(int fd) {
    if (fd >= 0) {
        close(fd);
    }
    return -1;
}

/**
 * @brief Creates a listening TCP socket on the loopback port.
 *
 * Returns the descriptor, or -1 with `errno` left by whichever call failed.
 */
static inline int net_listen_loopback(uint16_t port) {
    const int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) {
        return -1;
    }

    // Without this a socket still in TIME_WAIT from the previous test refuses
    // the bind, which would make each test's result depend on the one before.
    const int reuse = 1;
    if (setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &reuse, sizeof(reuse)) < 0) {
        return net_close(fd);
    }

    struct sockaddr_in addr;
    net_loopback_addr(&addr, port);
    if (bind(fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        return net_close(fd);
    }
    if (listen(fd, 4) < 0) {
        return net_close(fd);
    }

    return fd;
}

/**
 * @brief Connects a TCP socket to the loopback port.
 *
 * Returns the descriptor, or -1 with `errno` left by whichever call failed.
 */
static inline int net_connect_loopback(uint16_t port) {
    const int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) {
        return -1;
    }

    struct sockaddr_in addr;
    net_loopback_addr(&addr, port);
    if (connect(fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        return net_close(fd);
    }

    return fd;
}

/**
 * @brief Establishes a connected loopback pair, both ends in this process.
 *
 * `*client` is the connecting end and `*server` the accepted one. The listening
 * socket is closed before returning: it has done its job, and leaving it open
 * would keep the port claimed for the rest of the suite.
 *
 * Returns 0, or -1 with `errno` left by whichever call failed and both output
 * descriptors set to -1.
 *
 * The connect runs before the accept and both are blocking, which works only
 * because the listen backlog holds the connection until it is taken: the
 * kernel completes the handshake on the listener's behalf, so the connect
 * returns without anything having called accept yet.
 */
static inline int net_connected_pair(uint16_t port, int* client, int* server) {
    *client = -1;
    *server = -1;

    const int listener = net_listen_loopback(port);
    if (listener < 0) {
        return -1;
    }

    const int connected = net_connect_loopback(port);
    if (connected < 0) {
        net_close(listener);
        return -1;
    }

    const int accepted = accept(listener, NULL, NULL);
    if (accepted < 0) {
        net_close(connected);
        net_close(listener);
        return -1;
    }

    net_close(listener);
    *client = connected;
    *server = accepted;
    return 0;
}
