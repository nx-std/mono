// The console half of the netloader protocol, as the host's `nxlink` (and
// `cargo nx link`, which speaks the same protocol) drives it:
//
//   1. The host broadcasts "nxboot" over UDP to find a console willing to
//      receive; the console answers "bootnx", and the host takes the address
//      that reply came from as the console's.
//   2. The host connects over TCP and sends the file name, then the file's
//      length, and waits for a status word before sending anything more.
//   3. The file arrives as a sequence of length-prefixed deflate chunks; the
//      console inflates them into the file it reserved, and answers with a
//      second status word once the stream ends.
//   4. The host sends the command line as a run of NUL-terminated arguments.
//
// Every length and status word on the wire is a little-endian 32-bit integer,
// which is the console's own layout, so they are read and written as plain
// `uint32_t`/`int32_t` rather than being byte-swapped.

#include "netloader.h"

#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <netinet/in.h>
#include <poll.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <strings.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <unistd.h>
#include <zlib.h>

/** @brief The message a host broadcasts to find a console. */
#define NETLOADER_PING "nxboot"

/** @brief The answer that tells the host where the console is. */
#define NETLOADER_PONG "bootnx"

/**
 * @brief How long one read waits before the transfer is given up on.
 *
 * A host that stops mid-transfer would otherwise leave the runner blocked with
 * no way back to the screen short of rebooting the console.
 */
#define NETLOADER_RECV_TIMEOUT_MS 10000

/** @brief The largest deflate chunk the host is allowed to announce. */
#define NETLOADER_CHUNK_SIZE (16 * 1024)

/** @brief The largest command line the host is allowed to send. */
#define NETLOADER_ARGS_SIZE 3072

/** @brief The status words the protocol defines for a transfer that cannot start. */
enum {
    NETLOADER_RESPONSE_OK = 0,
    NETLOADER_RESPONSE_CANNOT_CREATE_FILE = -1,
    NETLOADER_RESPONSE_NOT_ENOUGH_SPACE = -2,
    NETLOADER_RESPONSE_NOT_A_PROGRAM = -3,
};

/**
 * @brief The inflate scratch buffers.
 *
 * Kept out of the stack because the runner runs the transfer on the main
 * thread, whose stack it shares with the console's own drawing.
 */
static uint8_t g_compressed[NETLOADER_CHUNK_SIZE];
static uint8_t g_plain[NETLOADER_CHUNK_SIZE];

/** @brief Records why a transfer did not complete. */
static void fail(NetloaderOutcome* out, const char* format, ...)
{
    va_list args;
    va_start(args, format);
    vsnprintf(out->error, sizeof(out->error), format, args);
    va_end(args);
}

static bool set_nonblocking(int fd)
{
    const int flags = fcntl(fd, F_GETFL);
    if (flags == -1) {
        return false;
    }
    return fcntl(fd, F_SETFL, flags | O_NONBLOCK) == 0;
}

/**
 * @brief Why the last read gave up.
 *
 * A read is reached through several callers and reports to none of them beyond having
 * failed, yet the difference between a host that went away, one that stopped mid-message
 * and one that sent something unusable is the whole of a diagnosis. Leaving the reason
 * here lets whichever caller describes the failure say which of them it was.
 *
 * Written only on the failing path, and read only by the caller that is already giving
 * up, so the one buffer cannot be overwritten between being set and being reported.
 */
static char g_read_failure[96];

/** @brief Records why a read gave up, for the message the caller is about to build. */
static void read_failed(const char* format, ...)
{
    va_list args;
    va_start(args, format);
    vsnprintf(g_read_failure, sizeof(g_read_failure), format, args);
    va_end(args);
}

/** @brief Reads exactly `size` bytes, or gives up. */
static bool recv_exact(int fd, void* buffer, size_t size)
{
    uint8_t* dst = buffer;
    size_t left = size;

    while (left > 0) {
        struct pollfd waiting = { .fd = fd, .events = POLLIN };
        const int ready = poll(&waiting, 1, NETLOADER_RECV_TIMEOUT_MS);
        if (ready == 0) {
            read_failed("nothing arrived for %d ms, %zu of %zu bytes in",
                        NETLOADER_RECV_TIMEOUT_MS, size - left, size);
            return false;
        }
        if (ready < 0) {
            read_failed("cannot wait on the connection: %s", strerror(errno));
            return false;
        }

        const ssize_t len = recv(fd, dst, left, 0);
        if (len == 0) {
            // The host closed the connection with the transfer unfinished. Told apart from
            // a malformed message because the two point at opposite ends of the problem:
            // this one says the host is not the one still talking to us.
            read_failed("the host closed the connection, %zu of %zu bytes in", size - left,
                        size);
            return false;
        }
        if (len < 0) {
            if (errno == EAGAIN || errno == EWOULDBLOCK) {
                continue;
            }
            read_failed("cannot read the connection: %s", strerror(errno));
            return false;
        }

        dst += len;
        left -= (size_t)len;
    }

    return true;
}

/** @brief Writes exactly `size` bytes, or gives up. */
static bool send_exact(int fd, const void* buffer, size_t size)
{
    const uint8_t* src = buffer;
    size_t left = size;

    while (left > 0) {
        struct pollfd waiting = { .fd = fd, .events = POLLOUT };
        if (poll(&waiting, 1, NETLOADER_RECV_TIMEOUT_MS) <= 0) {
            return false;
        }

        const ssize_t len = send(fd, src, left, 0);
        if (len == 0) {
            return false;
        }
        if (len < 0) {
            if (errno == EAGAIN || errno == EWOULDBLOCK) {
                continue;
            }
            return false;
        }

        src += len;
        left -= (size_t)len;
    }

    return true;
}

static bool recv_u32(int fd, uint32_t* value)
{
    return recv_exact(fd, value, sizeof(*value));
}

static bool send_response(int fd, int32_t response)
{
    return send_exact(fd, &response, sizeof(response));
}

/**
 * @brief Reads the name the host is sending, reduced to its last component.
 *
 * The runner writes everything into one directory of its own, so only the file
 * name itself is kept: no name a host sends can place a file anywhere else.
 */
static bool recv_file_name(int fd, char* name, size_t name_size)
{
    uint32_t length = 0;
    if (!recv_u32(fd, &length)) {
        return false;
    }
    if (length == 0 || length >= name_size) {
        read_failed("the name is %" PRIu32 " bytes, which is not between 1 and %zu", length,
                    name_size - 1);
        return false;
    }

    char sent[NETLOADER_PATH_SIZE];
    if (length >= sizeof(sent)) {
        read_failed("the name is %" PRIu32 " bytes, which does not fit %zu", length,
                    sizeof(sent) - 1);
        return false;
    }
    if (!recv_exact(fd, sent, length)) {
        return false;
    }
    sent[length] = '\0';

    const char* separator = strrchr(sent, '/');
    const char* base = separator != NULL ? separator + 1 : sent;
    if (base[0] == '\0') {
        read_failed("the name ends in a path separator, so it names no file");
        return false;
    }

    snprintf(name, name_size, "%s", base);
    return true;
}

static bool is_program_name(const char* name)
{
    const char* extension = strrchr(name, '.');
    return extension != NULL && strcasecmp(extension, ".nro") == 0;
}

/**
 * @brief Reserves the file the program will be written to.
 *
 * @return `NETLOADER_RESPONSE_OK`, or the status word that says what stopped it.
 */
static int32_t reserve_file(const char* name, uint32_t size, char* path, size_t path_size)
{
    if (!is_program_name(name)) {
        return NETLOADER_RESPONSE_NOT_A_PROGRAM;
    }

    if (mkdir(NETLOADER_DROP_DIR, 0777) != 0 && errno != EEXIST) {
        return NETLOADER_RESPONSE_CANNOT_CREATE_FILE;
    }

    const int written = snprintf(path, path_size, "%s/%s", NETLOADER_DROP_DIR, name);
    if (written < 0 || (size_t)written >= path_size) {
        return NETLOADER_RESPONSE_CANNOT_CREATE_FILE;
    }

    const int fd = open(path, O_CREAT | O_WRONLY | O_TRUNC, 0777);
    if (fd < 0) {
        return NETLOADER_RESPONSE_CANNOT_CREATE_FILE;
    }

    // Growing the file to its final length asks the card for the room up front,
    // so a card without it fails here rather than part-way through the
    // transfer. The write below reopens the file and takes the room again.
    const int reserved = ftruncate(fd, (off_t)size);
    close(fd);

    if (reserved != 0) {
        unlink(path);
        return NETLOADER_RESPONSE_NOT_ENOUGH_SPACE;
    }

    return NETLOADER_RESPONSE_OK;
}

/** @brief Inflates the chunk stream into `file` until the host ends it. */
static bool inflate_to_file(int fd, FILE* file, const char* name, size_t total,
                            NetloaderProgressFn on_progress, void* progress_ctx)
{
    z_stream stream = {
        .zalloc = Z_NULL,
        .zfree = Z_NULL,
        .opaque = Z_NULL,
    };
    if (inflateInit(&stream) != Z_OK) {
        return false;
    }

    size_t written = 0;
    int ret = Z_OK;

    do {
        uint32_t chunk_size = 0;
        if (!recv_u32(fd, &chunk_size) || chunk_size > sizeof(g_compressed)) {
            inflateEnd(&stream);
            return false;
        }
        if (!recv_exact(fd, g_compressed, chunk_size)) {
            inflateEnd(&stream);
            return false;
        }

        stream.avail_in = chunk_size;
        stream.next_in = g_compressed;

        do {
            stream.avail_out = sizeof(g_plain);
            stream.next_out = g_plain;

            ret = inflate(&stream, Z_NO_FLUSH);
            if (ret != Z_OK && ret != Z_STREAM_END) {
                inflateEnd(&stream);
                return false;
            }

            const size_t have = sizeof(g_plain) - stream.avail_out;
            if (fwrite(g_plain, 1, have, file) != have) {
                inflateEnd(&stream);
                return false;
            }

            written += have;
            if (on_progress != NULL) {
                on_progress(name, written, total, progress_ctx);
            }
        } while (stream.avail_out == 0);
    } while (ret != Z_STREAM_END);

    inflateEnd(&stream);
    return true;
}

/**
 * @brief Appends one argument to the command line, quoted.
 *
 * The runtime that parses this line splits on spaces and honours double quotes,
 * so quoting every argument keeps one holding a space in one piece.
 *
 * @return `false` when the argument does not fit, leaving the line as it was.
 */
static bool append_arg(char* cmdline, size_t cmdline_size, size_t* used, const char* arg)
{
    const char* separator = *used > 0 ? " " : "";
    const size_t left = cmdline_size - *used;

    const int written = snprintf(cmdline + *used, left, "%s\"%s\"", separator, arg);
    if (written < 0 || (size_t)written >= left) {
        cmdline[*used] = '\0';
        return false;
    }

    *used += (size_t)written;
    return true;
}

/**
 * @brief Reads the arguments the host sent and builds the command line.
 *
 * The line starts with the program's own path, the way a loader is expected to
 * pass it, and ends with the token the runtime reads the host's address out of:
 * a program that finds it there can send its output back to the host instead of
 * only to the screen. The token has to be last, so the arguments the host sent
 * go in between.
 */
static bool build_cmdline(int fd, const char* path, const char* extra_arg, struct in_addr host,
                          char* cmdline, size_t cmdline_size)
{
    uint32_t length = 0;
    if (!recv_u32(fd, &length)) {
        return false;
    }

    char args[NETLOADER_ARGS_SIZE];
    if (length > sizeof(args)) {
        return false;
    }
    if (length > 0 && !recv_exact(fd, args, length)) {
        return false;
    }

    size_t used = 0;
    if (!append_arg(cmdline, cmdline_size, &used, path)) {
        return false;
    }

    if (extra_arg != NULL && !append_arg(cmdline, cmdline_size, &used, extra_arg)) {
        return false;
    }

    size_t offset = 0;
    while (offset < length) {
        const char* arg = &args[offset];
        const size_t arg_len = strnlen(arg, length - offset);
        if (arg_len == 0 || arg_len == length - offset) {
            // Either the padding past the last argument, or a final one the
            // host did not terminate; there is nothing more to read either way.
            break;
        }

        if (!append_arg(cmdline, cmdline_size, &used, arg)) {
            // The line is full. Dropping the rest keeps the program launchable,
            // which is worth more than the arguments that did not fit.
            break;
        }

        offset += arg_len + 1;
    }

    // Bare, unlike every argument before it. The runtime reads this token off
    // the raw line rather than out of the parsed arguments, and looks for the
    // last whitespace-delimited word to be the address and the marker and
    // nothing else; a pair of quotes around it is two characters too many and
    // the host goes unrecorded.
    const size_t left = cmdline_size - used;
    const int written =
        snprintf(cmdline + used, left, " %08" PRIx32 "_NXLINK_", (uint32_t)host.s_addr);
    if (written < 0 || (size_t)written >= left) {
        // Without the token the program cannot reach the host, but it still
        // runs, so the line it has is worth more than no line at all.
        cmdline[used] = '\0';
    }

    return true;
}

/** @brief Runs the transfer over an accepted connection. */
static NetloaderStatus receive_program(int fd, struct in_addr host, NetloaderOutcome* out,
                                       const char* extra_arg,
                                       NetloaderProgressFn on_progress, void* progress_ctx)
{
    char name[NETLOADER_PATH_SIZE];
    if (!recv_file_name(fd, name, sizeof(name))) {
        fail(out, "no usable file name: %s", g_read_failure);
        return NETLOADER_FAILED;
    }

    uint32_t size = 0;
    if (!recv_u32(fd, &size)) {
        fail(out, "%s: no file length: %s", name, g_read_failure);
        return NETLOADER_FAILED;
    }

    const int32_t response = reserve_file(name, size, out->path, sizeof(out->path));

    // The host waits for this word before sending anything else, so it goes out
    // whether the file could be reserved or not.
    if (!send_response(fd, response)) {
        fail(out, "%s: the host stopped listening", name);
        return NETLOADER_FAILED;
    }
    if (response != NETLOADER_RESPONSE_OK) {
        fail(out, "%s: cannot receive it here (%" PRId32 ")", name, response);
        return NETLOADER_FAILED;
    }

    FILE* file = fopen(out->path, "wb");
    if (file == NULL) {
        fail(out, "%s: cannot open it for writing", name);
        return NETLOADER_FAILED;
    }

    const bool transferred = inflate_to_file(fd, file, name, size, on_progress, progress_ctx);
    fclose(file);

    if (!transferred) {
        // A part-written program would be launchable and wrong, which is worse
        // than not having it at all.
        unlink(out->path);
        fail(out, "%s: transfer incomplete: %s", name, g_read_failure);
        return NETLOADER_FAILED;
    }

    if (!send_response(fd, NETLOADER_RESPONSE_OK)) {
        fail(out, "%s: the host stopped listening", name);
        return NETLOADER_FAILED;
    }

    if (!build_cmdline(fd, out->path, extra_arg, host, out->cmdline, sizeof(out->cmdline))) {
        fail(out, "%s: no command line: %s", name, g_read_failure);
        return NETLOADER_FAILED;
    }

    return NETLOADER_RECEIVED;
}

bool netloader_open(NetloaderServer* server)
{
    server->discovery_fd = -1;
    server->listen_fd = -1;

    struct sockaddr_in address = {
        .sin_family = AF_INET,
        .sin_port = htons(NETLOADER_SERVER_PORT),
        .sin_addr = { .s_addr = htonl(INADDR_ANY) },
    };

    server->discovery_fd = socket(AF_INET, SOCK_DGRAM, 0);
    if (server->discovery_fd < 0) {
        goto failed;
    }
    if (bind(server->discovery_fd, (struct sockaddr*)&address, sizeof(address)) != 0) {
        goto failed;
    }
    if (!set_nonblocking(server->discovery_fd)) {
        goto failed;
    }

    server->listen_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (server->listen_fd < 0) {
        goto failed;
    }

    // A run that ended with a connection still in the kernel's hands would
    // otherwise leave the port unusable for as long as that connection lingers,
    // and the runner is relaunched after every program it hands off.
    const uint32_t reuse = 1;
    if (setsockopt(server->listen_fd, SOL_SOCKET, SO_REUSEADDR, &reuse, sizeof(reuse)) != 0) {
        goto failed;
    }
    if (bind(server->listen_fd, (struct sockaddr*)&address, sizeof(address)) != 0) {
        goto failed;
    }
    if (!set_nonblocking(server->listen_fd)) {
        goto failed;
    }
    if (listen(server->listen_fd, 1) != 0) {
        goto failed;
    }

    return true;

failed:
    netloader_close(server);
    return false;
}

void netloader_close(NetloaderServer* server)
{
    if (server->discovery_fd >= 0) {
        close(server->discovery_fd);
        server->discovery_fd = -1;
    }
    if (server->listen_fd >= 0) {
        close(server->listen_fd);
        server->listen_fd = -1;
    }
}

bool netloader_reopen(NetloaderServer* server)
{
    netloader_close(server);
    return netloader_open(server);
}

bool netloader_answer_discovery(const NetloaderServer* server)
{
    char ping[16];
    struct sockaddr_in host;
    socklen_t host_size = sizeof(host);

    const ssize_t len =
        recvfrom(server->discovery_fd, ping, sizeof(ping), 0, (struct sockaddr*)&host, &host_size);

    // Nothing waiting is the ordinary case on a socket nobody is pinging; anything else is
    // the socket itself having gone, which no amount of asking again will mend.
    if (len < 0) {
        return errno == EAGAIN || errno == EWOULDBLOCK;
    }

    // A datagram that is not the ping is somebody else's; the socket is still good.
    if (len < (ssize_t)strlen(NETLOADER_PING)) {
        return true;
    }
    if (memcmp(ping, NETLOADER_PING, strlen(NETLOADER_PING)) != 0) {
        return true;
    }

    // The host listens for the answer on a port of its own, not on the one it
    // asked from.
    host.sin_family = AF_INET;
    host.sin_port = htons(NETLOADER_CLIENT_PORT);
    sendto(server->discovery_fd, NETLOADER_PONG, strlen(NETLOADER_PONG), 0,
           (struct sockaddr*)&host, sizeof(host));

    return true;
}

NetloaderStatus netloader_receive(const NetloaderServer* server, NetloaderOutcome* out,
                                  const char* extra_arg, NetloaderProgressFn on_progress,
                                  void* progress_ctx)
{
    memset(out, 0, sizeof(*out));

    struct sockaddr_in host;
    socklen_t host_size = sizeof(host);

    const int fd = accept(server->listen_fd, (struct sockaddr*)&host, &host_size);
    if (fd < 0) {
        if (errno == EAGAIN || errno == EWOULDBLOCK) {
            return NETLOADER_IDLE;
        }
        // Nobody waiting is the ordinary case; anything else means the socket itself is
        // gone rather than this one attempt having failed, and nothing will arrive on it
        // again.
        fail(out, "the network went away: %s", strerror(errno));
        return NETLOADER_SERVER_LOST;
    }

    // Whether an accepted socket inherits the listening socket's non-blocking
    // mode is left open by the standard, and the reads below poll for
    // themselves, so it is set here rather than assumed either way.
    if (!set_nonblocking(fd)) {
        close(fd);
        fail(out, "cannot configure the connection: %s", strerror(errno));
        return NETLOADER_FAILED;
    }

    const NetloaderStatus status =
        receive_program(fd, host.sin_addr, out, extra_arg, on_progress, progress_ctx);
    close(fd);
    return status;
}
