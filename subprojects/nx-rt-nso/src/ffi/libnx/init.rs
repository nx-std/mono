//! Runtime startup and exit FFI (NSO).

use core::ffi::c_void;

use nx_svc::thread::Handle as ThreadHandle;

use crate::{
    env::LoaderReturnFn,
    init,
};

/// Brings the process up to the point `main` can be entered.
///
/// Corresponds to `__libnx_init()` in `runtime/init.c`, and is called by the
/// `.crt0` once the image has relocated itself.
///
/// The first argument is the homebrew loader's configuration block, which only
/// a homebrew NRO is given; the process manager passes a zero in its place.
/// Upstream reads that zero as "this process is an NSO" and forks on it. Here
/// there is nothing to fork: this is the NSO entry crate, so the argument is
/// ignored rather than tested.
///
/// Upstream declares this weak so a program can replace the whole of startup.
/// Aliasing it here takes that away; what a program keeps is the `userAppInit`
/// hook the service bring-up still calls.
///
/// The third argument is the loader-return function, which is not a return
/// path for this kind either: an NSO leaves through the process-exit syscall,
/// installed during environment bring-up. It is ignored here for the same
/// reason `envSetup` ignores it.
///
/// # Safety
///
/// Must be called once, on the startup thread, with `main_thread` the
/// kernel-supplied main-thread handle the process launch passed the `.crt0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nso__libnx_init(
    _ctx: *const c_void,
    main_thread: u32,
    _saved_lr: LoaderReturnFn,
) {
    // SAFETY: the caller guarantees `main_thread` is the handle the kernel
    // supplied for this process's main thread.
    let main_thread = ThreadHandle::from_raw_unchecked(main_thread);

    // SAFETY: the caller guarantees this runs once, on the startup thread.
    unsafe { init::init(main_thread) }
}

/// Closes what [`__nx_rt_nso__libnx_init`] opened and leaves the process.
///
/// Corresponds to `__libnx_exit()` in `runtime/init.c`. newlib reaches it
/// through a weak reference in `__syscall_exit`, which tests the address
/// before calling: aliasing the symbol here is what makes that test pass, and
/// a build that did not would spin in the `for (;;)` behind it.
///
/// # Safety
///
/// Must be called at most once, on the thread that is exiting, with every
/// service the startup sequence opened still open.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nso__libnx_exit(status: i32) -> ! {
    // SAFETY: the caller guarantees the exiting-thread contract above.
    unsafe { init::exit(status) }
}
