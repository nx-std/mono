/* Linker script — socket driver initialization override.
 *
 * Redirects libnx's `socketInitialize` to the Rust FFI in nx-rt-core. The rest
 * of the socket driver is overridden by nx-sys-net's own script; this one
 * symbol is separated out because its C contract is that the interface
 * revision follows the running firmware, and the firmware version is held by
 * the runtime, which nx-sys-net may not depend on.
 *
 * This fragment is gated by use_nx_sys_net on the consumer side: the
 * __nx_rt_core__libnx_socket_initialize symbol exists only when the `sys-net`
 * Cargo feature is active.
 *
 * Organization:
 *   - Rust FFI:  subprojects/nx-rt-core/src/ffi/libnx/socket.rs
 *   - Driver:    subprojects/nx-sys-net/sys_net_override.ld
 */

EXTERN(__nx_rt_core__libnx_socket_initialize);

socketInitialize = __nx_rt_core__libnx_socket_initialize;
