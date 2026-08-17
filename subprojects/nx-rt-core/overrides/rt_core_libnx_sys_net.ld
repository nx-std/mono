/* Linker script — the socket driver symbols the runtime owns.
 *
 * The bulk of the socket driver is overridden by nx-sys-net's own script.
 * These are separated out because each needs something nx-sys-net may not
 * reach for:
 *
 *   - `socketInitialize` declares an interface revision that follows the
 *     running firmware, and the firmware version is held by the runtime.
 *   - The five descriptor hand-offs each dispatch a command at an object the
 *     C caller owns — an ISslConnection or an IRequest — so they need the
 *     service crates that speak those interfaces, and two of them are gated on
 *     the running firmware as well.
 *
 * This fragment is gated by use_nx_sys_net on the consumer side: the
 * __nx_rt_core__libnx_socket_* symbols exist only when the `sys-net` Cargo
 * feature is active.
 *
 * Organization:
 *   - Rust FFI:  subprojects/nx-rt-core/src/ffi/libnx/socket.rs
 *   - Driver:    subprojects/nx-sys-net/sys_net_override.ld
 */

EXTERN(__nx_rt_core__libnx_socket_initialize);

socketInitialize = __nx_rt_core__libnx_socket_initialize;

/* Descriptor hand-off to the network-interface service */
EXTERN(__nx_rt_core__libnx_socket_nifm_request_register_socket_descriptor);
EXTERN(__nx_rt_core__libnx_socket_nifm_request_unregister_socket_descriptor);

socketNifmRequestRegisterSocketDescriptor   = __nx_rt_core__libnx_socket_nifm_request_register_socket_descriptor;
socketNifmRequestUnregisterSocketDescriptor = __nx_rt_core__libnx_socket_nifm_request_unregister_socket_descriptor;

/* The console's own address, which the network-interface service answers.
 *
 * Upstream keeps this in runtime/resolver.c beside the name-resolution calls,
 * but it asks nifm rather than sfdnsres, so it sits with the other nifm work
 * here instead of with the resolver overrides in rt_core_libnx_net.ld. That
 * also keeps it on the option that already carries the nifm dependency.
 *
 * gethostname is left to libnx: its own body calls gethostid, so it reaches
 * this through the alias below without needing one of its own.
 */
EXTERN(__nx_rt_core__libnx_gethostid);

gethostid = __nx_rt_core__libnx_gethostid;
