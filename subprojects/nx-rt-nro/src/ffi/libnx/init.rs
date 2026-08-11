//! Runtime startup and exit FFI

use core::ptr::NonNull;

use nx_svc::thread::Handle as ThreadHandle;

use crate::{
    env::{
        ConfigEntry,
        LoaderReturnFn,
    },
    init,
};

/// Brings the process up to the point `main` can be entered.
///
/// Corresponds to `__libnx_init()` in `runtime/init.c`, and is called by the
/// `.crt0` once the image has relocated itself.
///
/// Upstream declares this weak so a program can replace the whole of startup.
/// Aliasing it here takes that away, on the same terms as
/// [`__nx_rt_nro__libnx_app_init`][super::app::__nx_rt_nro__libnx_app_init]:
/// what a program is left with is the `userAppInit` hook the sequence still
/// calls, and replacing startup outright is the system-module pattern.
///
/// # Panics
///
/// Panics when a service libnx treats as mandatory fails to open, which ends
/// the process. See the applet-init entry point for why a panic is sound on a
/// `__nx_*` symbol here.
///
/// # Safety
///
/// Must be called once, on the startup thread, with the arguments the homebrew
/// loader passed the `.crt0`: `ctx` addressing a `ConfigEntry` array
/// terminated by `EndOfList` or null, `main_thread` the kernel-supplied
/// main-thread handle, and `saved_lr` the loader-return function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_init(
    ctx: *const ConfigEntry,
    main_thread: u32,
    saved_lr: LoaderReturnFn,
) {
    // SAFETY: the caller guarantees `main_thread` is the handle the kernel
    // supplied for this process's main thread.
    let main_thread = ThreadHandle::from_raw_unchecked(main_thread);

    // SAFETY: the caller guarantees the loader-supplied arguments and that
    // this runs once, on the startup thread.
    unsafe { init::init(NonNull::new(ctx.cast_mut()), main_thread, saved_lr) }
}

/// Closes what [`__nx_rt_nro__libnx_init`] opened and returns to the loader.
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_exit(status: i32) -> ! {
    // SAFETY: the caller guarantees this runs at most once, on the thread that
    // is exiting, with every service the startup sequence opened still open.
    unsafe { init::exit(status) }
}
