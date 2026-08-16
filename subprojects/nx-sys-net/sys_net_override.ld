/*
 * Socket driver ABI override — redirects libnx's socket driver symbols to the
 * Rust FFI in `nx-sys-net` (`src/ffi/`).
 * libnx counterpart: `src/nx/source/runtime/devices/socket.c`.
 *
 * `read`, `write` and `close` are deliberately absent: they are descriptor
 * operations that libsysbase resolves through the descriptor table, which
 * dispatches them into `nx-sys-net`'s socket device without passing through
 * any symbol here.
 */

/* Driver lifecycle.
 * `socketInitialize` is absent: its interface revision follows the running
 * firmware, so the runtime owns that entry point. See
 * subprojects/nx-rt-core/overrides/rt_core_libnx_sys_net.ld. */
EXTERN(__nx_sys_net__socketExit);
EXTERN(__nx_sys_net__socketGetDefaultInitConfig);
EXTERN(__nx_sys_net__socketGetLastResult);

socketExit                 = __nx_sys_net__socketExit;
socketGetDefaultInitConfig = __nx_sys_net__socketGetDefaultInitConfig;
socketGetLastResult        = __nx_sys_net__socketGetLastResult;

/* The five descriptor hand-offs to the TLS and network-interface services are
 * absent: each dispatches a command at an object the C caller owns, and two of
 * them are gated on the running firmware, so the runtime owns those entry
 * points. See subprojects/nx-rt-core/overrides/rt_core_libnx_sys_net.ld. */

/* Establishing and naming sockets */
EXTERN(__nx_sys_net__socket);
EXTERN(__nx_sys_net__bind);
EXTERN(__nx_sys_net__connect);
EXTERN(__nx_sys_net__listen);
EXTERN(__nx_sys_net__accept);
EXTERN(__nx_sys_net__getsockname);
EXTERN(__nx_sys_net__getpeername);
EXTERN(__nx_sys_net__shutdown);
EXTERN(__nx_sys_net__sockatmark);
EXTERN(__nx_sys_net__socketpair);

socket      = __nx_sys_net__socket;
bind        = __nx_sys_net__bind;
connect     = __nx_sys_net__connect;
listen      = __nx_sys_net__listen;
accept      = __nx_sys_net__accept;
getsockname = __nx_sys_net__getsockname;
getpeername = __nx_sys_net__getpeername;
shutdown    = __nx_sys_net__shutdown;
sockatmark  = __nx_sys_net__sockatmark;
socketpair  = __nx_sys_net__socketpair;

/* Moving data */
EXTERN(__nx_sys_net__recv);
EXTERN(__nx_sys_net__recvfrom);
EXTERN(__nx_sys_net__send);
EXTERN(__nx_sys_net__sendto);
EXTERN(__nx_sys_net__recvmsg);
EXTERN(__nx_sys_net__sendmsg);
EXTERN(__nx_sys_net__recvmmsg);
EXTERN(__nx_sys_net__sendmmsg);

recv     = __nx_sys_net__recv;
recvfrom = __nx_sys_net__recvfrom;
send     = __nx_sys_net__send;
sendto   = __nx_sys_net__sendto;
recvmsg  = __nx_sys_net__recvmsg;
sendmsg  = __nx_sys_net__sendmsg;
recvmmsg = __nx_sys_net__recvmmsg;
sendmmsg = __nx_sys_net__sendmmsg;

/* Options, flags and kernel parameters */
EXTERN(__nx_sys_net__getsockopt);
EXTERN(__nx_sys_net__setsockopt);
EXTERN(__nx_sys_net__fcntl);
EXTERN(__nx_sys_net__ioctl);
EXTERN(__nx_sys_net__sysctl);
EXTERN(__nx_sys_net__sysctlbyname);
EXTERN(__nx_sys_net__sysctlnametomib);

getsockopt      = __nx_sys_net__getsockopt;
setsockopt      = __nx_sys_net__setsockopt;
fcntl           = __nx_sys_net__fcntl;
ioctl           = __nx_sys_net__ioctl;
sysctl          = __nx_sys_net__sysctl;
sysctlbyname    = __nx_sys_net__sysctlbyname;
sysctlnametomib = __nx_sys_net__sysctlnametomib;

/* Readiness */
EXTERN(__nx_sys_net__select);
EXTERN(__nx_sys_net__poll);

select = __nx_sys_net__select;
poll   = __nx_sys_net__poll;
