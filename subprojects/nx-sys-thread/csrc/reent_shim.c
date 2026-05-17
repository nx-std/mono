// newlib `_reent` shim for `nx-sys-thread`.
//
// Spawned threads need a real newlib reentrancy block so `errno` and stdio work
// on them (libnx behavioral parity — see IC-16). `sizeof(struct _reent)` and
// the `_REENT_INIT_PTR` initializer are devkitA64 newlib ABI details a
// pure-Rust crate cannot reproduce, so they live here in C: the Rust core
// (`MirrorLayout`/`create`) sizes and initializes the block exclusively through
// the two symbols below and never transcribes the layout itself.
//
// It also exposes a thin `errno` setter: `errno` resolves through newlib's
// `__errno()` to the calling thread's `_reent`, the same ABI detail kept in C.
//
// This translation unit is compiled by `meson.build` and linked into the final
// NRO only when a consumer enables the `nx-sys-thread` FFI override surface;
// when the surface is off the Rust side references neither symbol, so the
// linker drops the whole object.

#include <errno.h>
#include <stddef.h>
#include <sys/reent.h>

// 16-byte-aligned size of a newlib `_reent` block, matching libnx
// `threadCreate`'s `reent_sz` arithmetic (`thread.c`). Exported as data so the
// Rust layout reserves the block from this `sizeof` and can never drift from
// the ABI.
//
// Deliberately a linked runtime symbol, not a build-time constant: keeping it
// here means this `sizeof` and the `_REENT_INIT_PTR` initializer below are
// derived by one compiler from one `<sys/reent.h>` in a single translation
// unit, so they cannot disagree. A build-time probe (compile-and-extract or
// bindgen) would re-derive the layout through a second path that must stay in
// lockstep with the initializer — reintroducing the drift this shim prevents.
// The Rust side reads it once per spawn, so the load is not worth optimizing.
const size_t __nx_sys_thread_reent_size = (sizeof(struct _reent) + 0xF) & ~(size_t)0xF;

// Initializes a freshly reserved `_reent` block for a spawned thread.
//
// Mirrors libnx `threadCreate`: runs `_REENT_INIT_PTR` over the block, then
// inherits the creating thread's standard stream handles so the child shares
// stdin/stdout/stderr. `child` must point at `__nx_sys_thread_reent_size`
// writable bytes; the call runs on the creating thread while the child is
// still suspended.
void __nx_sys_thread_reent_init(struct _reent *child)
{
    _REENT_INIT_PTR(child);

    struct _reent *parent = __getreent();
    child->_stdin = parent->_stdin;
    child->_stdout = parent->_stdout;
    child->_stderr = parent->_stderr;
}

// Sets the calling thread's newlib `errno`.
//
// `errno` expands to `(*__errno())`, writing into the calling thread's
// `_reent` — an ABI detail kept in C alongside the `_reent` provisioning above.
// The `__syscall_nanosleep` adapter uses this on its failure path for libnx
// parity (`newlib.c`'s `__syscall_nanosleep` sets `errno = EINVAL`).
void __nx_sys_thread_set_errno(int code)
{
    errno = code;
}
