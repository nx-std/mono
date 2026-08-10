/* Linker script — resolver overrides that need the service-manager session.
 *
 * Redirects the five libnx resolver symbols that reach `sfdnsres` to the Rust
 * FFI in nx-rt-core. The rest of the resolver is overridden by nx-net's own
 * script; these five are separated out because acquiring `sfdnsres` needs the
 * process's service-manager session, and a process gets exactly one — which
 * the runtime holds, so nx-net cannot open its own.
 *
 * This fragment is gated by use_nx_net on the consumer side: the
 * __nx_rt_core__libnx_* symbols below exist only when the `net` Cargo feature
 * is active.
 *
 * Organization:
 *   - Rust FFI:  subprojects/nx-rt-core/src/ffi/libnx/net.rs
 *   - Resolver:  subprojects/nx-net/net_override.ld
 */

EXTERN(__nx_rt_core__libnx_getaddrinfo);
EXTERN(__nx_rt_core__libnx_getnameinfo);
EXTERN(__nx_rt_core__libnx_gethostbyname);
EXTERN(__nx_rt_core__libnx_gethostbyname2);
EXTERN(__nx_rt_core__libnx_gethostbyaddr);

getaddrinfo    = __nx_rt_core__libnx_getaddrinfo;
getnameinfo    = __nx_rt_core__libnx_getnameinfo;
gethostbyname  = __nx_rt_core__libnx_gethostbyname;
gethostbyname2 = __nx_rt_core__libnx_gethostbyname2;
gethostbyaddr  = __nx_rt_core__libnx_gethostbyaddr;
