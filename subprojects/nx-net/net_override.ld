/*
 * Resolver ABI override — redirects libnx's `netdb`/`getaddrinfo` symbols to
 * the Rust FFI in `nx-net` (`src/ffi.rs`).
 * libnx counterpart: `src/nx/source/runtime/resolver.c`.
 */

/*
 * The five that reach `sfdnsres` are absent: acquiring that service needs the
 * process's service-manager session, which the runtime owns, so those symbols
 * live with it. See subprojects/nx-rt-core/overrides/rt_core_libnx_net.ld.
 */

/* getaddrinfo family */
EXTERN(__nx_net__freeaddrinfo);
EXTERN(__nx_net__gai_strerror);

freeaddrinfo = __nx_net__freeaddrinfo;
gai_strerror = __nx_net__gai_strerror;

/* gethostby* family */
EXTERN(__nx_net__freehostent);
EXTERN(__nx_net__hstrerror);
EXTERN(__nx_net__herror);

freehostent = __nx_net__freehostent;
hstrerror   = __nx_net__hstrerror;
herror      = __nx_net__herror;
