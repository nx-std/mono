//! Module constructor / destructor glue.
//!
//! A dynamically loadable module has no `_start`: the `ro` service relocates
//! it into a host process, and the host then runs the module's static
//! constructors and, on unload, its destructors. The toolchain collects those
//! into the `.init_array` and `.fini_array` tables; the functions here walk
//! those tables and invoke each entry.
//!
//! This is the *only* startup work a module owns. The heap, environment,
//! Service Manager session, and applet identity all belong to the host
//! process — see the crate root for the module-versus-process split.

/// A C-ABI constructor or destructor, as stored in `.init_array` /
/// `.fini_array`.
type CtorFn = unsafe extern "C" fn();

// Bounds of the constructor/destructor tables. The linker emits these symbols
// at the start and end of `.init_array` / `.fini_array`; typing each as a
// `CtorFn` makes `&raw const` of the symbol a pointer to the first table slot.
unsafe extern "C" {
    static __init_array_start: CtorFn;
    static __init_array_end: CtorFn;
    static __fini_array_start: CtorFn;
    static __fini_array_end: CtorFn;
}

/// Runs the module's static constructors in `.init_array` order.
///
/// # Safety
///
/// - Must be called exactly once, after the `ro` service has applied the
///   module's relocations and before any other module code runs.
/// - Must run single-threaded — no other thread may observe partially
///   constructed module state.
/// - The `.init_array` bounds symbols must be supplied by the final link.
pub unsafe fn run_init_array() {
    let start: *const CtorFn = &raw const __init_array_start;
    let end: *const CtorFn = &raw const __init_array_end;

    let mut cur = start;
    while cur < end {
        // SAFETY: `cur` is within `[start, end)`, a valid `.init_array` slot
        // populated by the linker, so the read yields a real constructor.
        let ctor = unsafe { *cur };
        // SAFETY: `.init_array` entries are C-ABI constructors emitted by the
        // toolchain; running each once after relocation is their contract.
        unsafe { ctor() };
        // SAFETY: `cur < end`, so the next slot is still within the table
        // (one past `end` at most, which terminates the loop).
        cur = unsafe { cur.add(1) };
    }
}

/// Runs the module's static destructors in reverse `.fini_array` order.
///
/// # Safety
///
/// - Must be called exactly once, before the module is unloaded, and after
///   [`run_init_array`].
/// - Must run single-threaded — no other thread may observe module state
///   while it is being torn down.
/// - The `.fini_array` bounds symbols must be supplied by the final link.
pub unsafe fn run_fini_array() {
    let start: *const CtorFn = &raw const __fini_array_start;
    let end: *const CtorFn = &raw const __fini_array_end;

    let mut cur = end;
    while cur > start {
        // SAFETY: `cur > start`, so the preceding slot is within
        // `[start, end)` — a valid `.fini_array` slot.
        cur = unsafe { cur.sub(1) };
        // SAFETY: `cur` now points at a valid `.fini_array` slot, so the read
        // yields a real destructor.
        let dtor = unsafe { *cur };
        // SAFETY: `.fini_array` entries are C-ABI destructors emitted by the
        // toolchain; running each once before unload is their contract.
        unsafe { dtor() };
    }
}
