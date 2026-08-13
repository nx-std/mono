#pragma once

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// The port the server listens on, for both halves of the protocol.
#define NX_NETLOADER_SERVER_PORT 28280

/// How much room the received program's path gets.
#define NX_NETLOADER_PATH_SIZE 256

/// How much room the command line handed to the next program gets.
#define NX_NETLOADER_CMDLINE_SIZE 2048

/// How much room a failure reason gets.
#define NX_NETLOADER_ERROR_SIZE 192

/// Opaque struct for the bound sockets a host reaches this console through.
typedef struct NxNetloaderServer NxNetloaderServer;

/// What one receive attempt found.
typedef enum {
    /// Nobody is connecting; nothing was received.
    NX_NETLOADER_IDLE = 0,
    /// A program arrived and was written to the drop directory.
    NX_NETLOADER_RECEIVED = 1,
    /// A host connected but the transfer did not complete.
    NX_NETLOADER_FAILED = 2,
    /**
     * The sockets can no longer be listened on, and nothing will arrive until they are rebuilt.
     *
     * The console taking its network down, which it does whenever it sleeps, invalidates both
     * sockets while leaving the caller running and apparently well. Reported apart from a failed
     * transfer because the two ask opposite things of the caller: a failed transfer is over and the
     * next one may still arrive, whereas this one means no transfer can arrive again until
     * the server is freed and opened again.
     */
    NX_NETLOADER_SERVER_LOST = 3,
} NxNetloaderStatus;

/**
 * @brief What a receive attempt produced.
 *
 * Which fields carry anything depends on the status the attempt returned:
 * `NX_NETLOADER_RECEIVED` fills `path` and `cmdline`, `NX_NETLOADER_FAILED` fills `error`, and
 * `NX_NETLOADER_IDLE` fills none of them.
 */
typedef struct {
    /// Where the program was written.
    char path[NX_NETLOADER_PATH_SIZE];
    /// The command line to launch it with, quoted the way the runtime parses it.
    char cmdline[NX_NETLOADER_CMDLINE_SIZE];
    /// Why the transfer did not complete.
    char error[NX_NETLOADER_ERROR_SIZE];
} NxNetloaderOutcome;

/**
 * @brief Called as a transfer advances, so the caller can show progress.
 * @param[in] name The file name the host is sending.
 * @param[in] received How many bytes have been written so far.
 * @param[in] total How many bytes the whole file holds.
 * @param[in] ctx Whatever the caller passed to `__nx_netloader__server_receive`.
 */
typedef void (*NxNetloaderProgressFn)(const char* name, size_t received, size_t total, void* ctx);

/**
 * @brief Binds both sockets and starts listening.
 * @param[in] drop_dir The directory a received program is written to. Which directory that is
 *            belongs to the caller: every program it receives goes there under the file name alone,
 *            so nothing a host sends can name a file outside it.
 * @return The server, or `NULL` when either socket could not be bound, in which case nothing is
 *         left open.
 * @remark The caller is responsible for freeing the server with `__nx_netloader__server_free`.
 * @remark Rebuilding after the network goes away is a free followed by an open: there is no
 *         separate rebuild call, because one that consumed its argument would leave a caller
 *         holding nothing to retry with once it failed.
 */
NxNetloaderServer* __nx_netloader__server_open(const char* drop_dir);

/**
 * @brief Frees a `NxNetloaderServer`, closing both sockets.
 * @param[in] server The server to free.
 * @note If `server` is `NULL`, this function does nothing.
 */
void __nx_netloader__server_free(NxNetloaderServer* server);

/**
 * @brief Answers a pending discovery ping, if one has arrived.
 *
 * A host that was not told an address finds the console by broadcasting a ping and waiting for this
 * reply, so this has to be called regularly for as long as the caller is willing to be found.
 *
 * @param[in] server The server.
 * @return 0 when the socket is still good, and -1 when it has failed in a way waiting will not
 *         mend, which means the server must be freed and opened again before any host can find
 *         this console again.
 */
int32_t __nx_netloader__server_answer_discovery(NxNetloaderServer* server);

/**
 * @brief Receives one program, if a host is connecting.
 *
 * Returns immediately with `NX_NETLOADER_IDLE` when no host is connecting. Once one is, the
 * transfer runs to completion before returning, calling `on_progress` as it goes.
 *
 * @param[in] server The server.
 * @param[out] out What the attempt produced.
 * @param[in] extra_arg An argument to give the program ahead of the ones the host sent, or `NULL`
 *            for none. It is how the caller says something to every program it launches, whatever
 *            the host had to say.
 * @param[in] on_progress Called as the transfer advances, or `NULL` for no reporting.
 * @param[in] progress_ctx Passed back to `on_progress` unchanged.
 * @return What the attempt found.
 */
NxNetloaderStatus __nx_netloader__server_receive(NxNetloaderServer* server,
                                                NxNetloaderOutcome* out,
                                                const char* extra_arg,
                                                NxNetloaderProgressFn on_progress,
                                                void* progress_ctx);

#ifdef __cplusplus
}
#endif
