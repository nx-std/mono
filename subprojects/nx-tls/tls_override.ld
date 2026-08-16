/*
 * TLS socket hand-off ABI override — redirects the libnx socket driver's
 * `socketSslConnection*` symbols to the Rust FFI in `nx-tls` (`src/ffi.rs`).
 * libnx counterpart: `src/nx/source/runtime/devices/socket.c`.
 *
 * These live here rather than with the rest of the socket driver because each
 * dispatches a command at an `ISslConnection` the C caller owns, which the
 * driver crate does not know about. The rest of `ssl.c` is not ported yet, so
 * this script covers only the hand-offs.
 */

/* Giving a socket to a TLS connection, and taking it back */
EXTERN(__nx_tls__socketSslConnectionSetSocketDescriptor);
EXTERN(__nx_tls__socketSslConnectionGetSocketDescriptor);
EXTERN(__nx_tls__socketSslConnectionSetDtlsSocketDescriptor);

socketSslConnectionSetSocketDescriptor     = __nx_tls__socketSslConnectionSetSocketDescriptor;
socketSslConnectionGetSocketDescriptor     = __nx_tls__socketSslConnectionGetSocketDescriptor;
socketSslConnectionSetDtlsSocketDescriptor = __nx_tls__socketSslConnectionSetDtlsSocketDescriptor;
