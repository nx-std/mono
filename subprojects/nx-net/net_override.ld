/*
 * Resolver ABI override — redirects libnx's `netdb`/`getaddrinfo` symbols to
 * the Rust FFI in `nx-net` (`src/ffi.rs`).
 * libnx counterpart: `src/nx/source/runtime/resolver.c`.
 */

/* getaddrinfo family */
EXTERN(__nx_net__getaddrinfo);
EXTERN(__nx_net__freeaddrinfo);
EXTERN(__nx_net__gai_strerror);
EXTERN(__nx_net__getnameinfo);

getaddrinfo  = __nx_net__getaddrinfo;
freeaddrinfo = __nx_net__freeaddrinfo;
gai_strerror = __nx_net__gai_strerror;
getnameinfo  = __nx_net__getnameinfo;

/* gethostby* family */
EXTERN(__nx_net__gethostbyname);
EXTERN(__nx_net__gethostbyname2);
EXTERN(__nx_net__gethostbyaddr);
EXTERN(__nx_net__freehostent);
EXTERN(__nx_net__hstrerror);
EXTERN(__nx_net__herror);

gethostbyname  = __nx_net__gethostbyname;
gethostbyname2 = __nx_net__gethostbyname2;
gethostbyaddr  = __nx_net__gethostbyaddr;
freehostent    = __nx_net__freehostent;
hstrerror      = __nx_net__hstrerror;
herror         = __nx_net__herror;
