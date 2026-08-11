//! Environment / loader bring-up FFI (NSO).
//!
//! Redirects the NSO-specific `libnx` runtime entry point `envSetup` to
//! `nx-rt-nso`. The kind-agnostic runtime symbols: heap init, main-thread
//! TLS, the environment read accessors, the HOS-version API: are owned by
//! `nx-rt-core`'s FFI surface.

use core::ffi::c_void;

use nx_svc::thread::Handle as ThreadHandle;

use crate::env::{
    self,
    LoaderReturnFn,
};

/// Brings up the NSO runtime environment state.
///
/// Corresponds to `envSetup()` in `env.h`. An NSO process receives no
/// homebrew-loader configuration block, so the `ctx` argument: meaningful
/// only for a homebrew NRO: is ignored. Neither is the loader-return function
/// a return path for this kind: an NSO leaves through the process-exit
/// syscall, which [`env::setup`] installs itself. That leaves the
/// kernel-supplied main-thread handle as the only input reaching it.
///
/// # Safety
///
/// `main_thread` must be the kernel-supplied main-thread handle passed by the
/// NSO crt0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nso__libnx_env_setup(
    _ctx: *const c_void,
    main_thread: u32,
    _saved_lr: LoaderReturnFn,
) {
    // SAFETY: per this function's contract, `main_thread` is the valid
    // kernel-supplied main-thread handle.
    let main_thread = ThreadHandle::from_raw_unchecked(main_thread);
    env::setup(main_thread);
}
