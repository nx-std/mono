#pragma once

#include <stdbool.h>
#include <stddef.h>

/**
 * @brief The port the server listens on.
 *
 * One number covers both halves of the protocol: a UDP socket that answers the
 * host's discovery ping, and a TCP socket that accepts the transfer itself.
 */
#define NETLOADER_SERVER_PORT 28280

/** @brief The UDP port a discovery reply is sent back to. */
#define NETLOADER_CLIENT_PORT 28771

/**
 * @brief How much room the received program's path gets.
 *
 * The runner hands this path to `envSetNextLoad`, which copies it into a buffer
 * the process loader owns without checking how long it is. That buffer is 512
 * bytes, so staying under half of it leaves no way to overrun it.
 */
#define NETLOADER_PATH_SIZE 256

/**
 * @brief How much room the command line handed to the next program gets.
 *
 * Sized to the buffer the process loader copies it into, for the same reason as
 * the path above: `envSetNextLoad` does not bound the copy, so the bound has to
 * live here.
 */
#define NETLOADER_CMDLINE_SIZE 2048

/** @brief How much room a failure reason gets. */
#define NETLOADER_ERROR_SIZE 192

/**
 * @brief The directory received programs are written to.
 *
 * The runner owns this one directory and writes every program it receives into
 * it under the file name alone, so nothing a host sends can name a file outside
 * it.
 */
#define NETLOADER_DROP_DIR "sdmc:/switch/nx-tests"

/** @brief The listening sockets, once they are bound. */
typedef struct {
    /** Answers discovery pings so the host can find this console. */
    int discovery_fd;
    /** Accepts the transfer connection. */
    int listen_fd;
} NetloaderServer;

/** @brief What one receive attempt found. */
typedef enum {
    /** Nobody is connecting; nothing was received. */
    NETLOADER_IDLE,
    /** A program arrived and was written to the drop directory. */
    NETLOADER_RECEIVED,
    /** A host connected but the transfer did not complete. */
    NETLOADER_FAILED,
    /**
     * The sockets can no longer be listened on, and nothing will arrive until they are
     * rebuilt.
     *
     * The console taking its network down, which it does whenever it sleeps, invalidates
     * both sockets while leaving the runner running and apparently well. Reported apart
     * from a failed transfer because the two ask opposite things of the caller: a failed
     * transfer is over and the next one may still arrive, whereas this one means no
     * transfer can arrive again until `netloader_reopen` succeeds.
     */
    NETLOADER_SERVER_LOST,
} NetloaderStatus;

/**
 * @brief What a receive attempt produced.
 *
 * Which fields carry anything depends on the status the attempt returned:
 * `NETLOADER_RECEIVED` fills `path` and `cmdline`, `NETLOADER_FAILED` fills
 * `error`, and `NETLOADER_IDLE` fills none of them.
 */
typedef struct {
    /** Where the program was written. */
    char path[NETLOADER_PATH_SIZE];
    /** The command line to launch it with, quoted the way the runtime parses it. */
    char cmdline[NETLOADER_CMDLINE_SIZE];
    /** Why the transfer did not complete. */
    char error[NETLOADER_ERROR_SIZE];
} NetloaderOutcome;

/**
 * @brief Called as a transfer advances, so the caller can show progress.
 *
 * @param name The file name the host is sending.
 * @param received How many bytes have been written so far.
 * @param total How many bytes the whole file holds.
 * @param ctx Whatever the caller passed to `netloader_receive`.
 */
typedef void (*NetloaderProgressFn)(const char* name, size_t received, size_t total, void* ctx);

/**
 * @brief Binds both sockets and starts listening.
 *
 * @return `false` when either socket could not be bound, in which case nothing
 *         is left open.
 */
bool netloader_open(NetloaderServer* server);

/** @brief Closes whatever `netloader_open` opened. */
void netloader_close(NetloaderServer* server);

/**
 * @brief Closes both sockets and binds them again.
 *
 * What a console that has slept needs: its network went away and came back, and the
 * sockets bound to the old one answer nothing. Binding is what fails while the network is
 * still down, so a caller retries rather than treating one failure as final.
 *
 * @return `false` when the sockets could not be bound, in which case nothing is left open
 *         and the call can simply be made again later.
 */
bool netloader_reopen(NetloaderServer* server);

/**
 * @brief Answers a pending discovery ping, if one has arrived.
 *
 * A host that was not told an address finds the console by broadcasting a ping
 * and waiting for this reply, so this has to be called regularly for as long as
 * the runner is willing to be found.
 *
 * @return `false` when the socket has failed in a way waiting will not mend, which means
 *         the server needs rebuilding before any host can find this console again.
 */
bool netloader_answer_discovery(const NetloaderServer* server);

/**
 * @brief Receives one program, if a host is connecting.
 *
 * Returns immediately with `NETLOADER_IDLE` when no host is connecting. Once
 * one is, the transfer runs to completion before returning, calling
 * `on_progress` as it goes.
 *
 * @param extra_arg An argument to give the program ahead of the ones the host
 *        sent, or `NULL` for none. It is how the caller says something to every
 *        program it launches, whatever the host had to say.
 */
NetloaderStatus netloader_receive(const NetloaderServer* server, NetloaderOutcome* out,
                                  const char* extra_arg, NetloaderProgressFn on_progress,
                                  void* progress_ctx);
