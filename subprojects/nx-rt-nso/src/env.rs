//! # Runtime Environment State (NSO)
//!
//! Brings up the kind-agnostic [runtime environment state][nx_rt_core::env]
//! for an NSO process launched by the process manager (`pm`).
//!
//! Unlike a homebrew NRO — which the homebrew loader hands a `ConfigEntry`
//! array describing its heap override, command-line arguments, service
//! overrides, applet type, and HOS version — an NSO process receives no such
//! configuration block. [`setup`] therefore has nothing to parse: it records
//! only what the NSO launch ABI guarantees directly — that the process runs
//! as an NSO, that every supervisor call is available, and the kernel-supplied
//! main-thread handle and loader-return function.
//!
//! The read accessors and the HOS-version / main-thread submodules are
//! re-exported from [`nx_rt_core::env`] so callers reach them through
//! `crate::env::*`, exactly as they do for the homebrew-NRO entry crate.

use nx_rt_core::env::init_once;
pub use nx_rt_core::env::{
    AccountUid, AppletType, LoaderReturnFn, ServiceName, ServiceOverride, SyscallHints,
    applet_type, applet_workaround, argv, exit_func_ptr, has_next_load, heap_override, hos_version,
    is_nso, last_load_result, loader_info, main_thread, main_thread_handle, own_process_handle,
    random_seed, service_overrides, set_exit_func_ptr, set_next_load, syscall_hints,
    user_id_storage,
};
use nx_svc::thread::Handle as ThreadHandle;

/// Populates the runtime environment state for an NSO process.
///
/// Runs exactly once — repeat calls are no-ops. An NSO has no homebrew-loader
/// configuration block, so there is nothing to parse: no heap override, no
/// `argv` pointer, and no loader-supplied service overrides. The bring-up
/// records that the process is an NSO, marks every supervisor call available
/// (the NSO ABI grants the full set), and seeds the main-thread handle and
/// loader-return function from the kernel-supplied startup arguments.
pub fn setup(main_thread: ThreadHandle, saved_lr: LoaderReturnFn) {
    init_once(|state| {
        // An NSO process is unconditionally an NSO.
        state.is_nso = true;
        // Seed the main-thread handle from the kernel-supplied argument.
        state.main_thread_handle = Some(main_thread);
        // An NSO is granted the full supervisor-call set; there are no loader
        // hints restricting it, so mark every syscall available.
        state.syscall_hints = Some(SyscallHints::all_available());

        set_exit_func_ptr(saved_lr);
    });
}
