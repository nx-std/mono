//! The startup sequence the `.crt0` hands off to, and the way back out.
//!
//! Ports libnx's `__libnx_init` and `__libnx_exit`: the outermost pair of the
//! C runtime, sitting between the assembly that self-relocates the image and
//! the program's `main`. [`init`] is the whole of startup, from the loader's
//! configuration block through to the static constructors; [`exit`] is the
//! return path to the loader. The steps themselves belong elsewhere, most of
//! them to other crates; what lives here is the order they run in.
//!
//! # Why the order lives in this crate
//!
//! Three of the eight startup steps are this crate's own: parsing the homebrew
//! loader's configuration block, building the command line out of it, and the
//! service bring-up in [`crate::app`]. [`nx_rt_core`] cannot sequence steps it
//! has no way to name, because the dependency runs the other way, and the
//! third does not exist for every output kind: an NSO leaves `__appInit` with
//! libnx. Passing the three in as function pointers would buy a shared
//! sequence at the cost of the one thing this module is for, which is reading
//! as the startup order it encodes.
//!
//! # A step this build did not take over
//!
//! Two steps live in crates this one depends on only optionally, behind the
//! `sys-virtmem` and `sys-thread` features, so a build with the matching Meson
//! option off reaches them with the crate absent. Such a step calls libnx's
//! entry point by its C name rather than skipping, for the reason
//! [`crate::app`] gives at greater length: skipping would leave the work
//! undone, and with the feature off that C name can only be libnx's own
//! implementation, because the linker fragment that would redirect it is added
//! by the same Meson option that sets the feature.
//!
//! The last step is not one of those. `__libc_init_array` is newlib's, not
//! libnx's, and nothing in this workspace replaces it, so it is always reached
//! through C.

use core::ptr::NonNull;

use nx_svc::thread::Handle as ThreadHandle;

use crate::{
    app,
    argv,
    env::{
        self,
        ConfigEntry,
        LoaderReturnFn,
    },
};

/// Brings the process up to the point `main` can be entered.
///
/// Corresponds to `__libnx_init()` in `runtime/init.c`.
///
/// # Panics
///
/// Panics when a service the default bring-up treats as mandatory fails to
/// open, which ends the process. See [`crate::app::init`], which owns that
/// set and the reason a panic is how this workspace ends where libnx aborted.
///
/// # Safety
///
/// Must be called exactly once, on the startup thread, with the arguments the
/// homebrew loader handed the `.crt0`: `ctx` addressing the configuration
/// block, `main_thread` the handle of the process main thread, and `saved_lr`
/// the address to return to the loader through.
pub unsafe fn init(
    ctx: Option<NonNull<ConfigEntry>>,
    main_thread: ThreadHandle,
    saved_lr: LoaderReturnFn,
) {
    // SAFETY: the caller guarantees the loader-supplied arguments, and that
    // this runs once on the startup thread before anything reads back the
    // environment it fills.
    unsafe { env::setup(ctx, main_thread, saved_lr) };

    // SAFETY: the environment is parsed, so the main thread's handle is known,
    // and no thread-local has been touched yet.
    unsafe { env::main_thread::setup() };

    setup_virtmem();
    nx_rt_core::init::setup_heap();
    init_main_thread();

    // SAFETY: the heap is up, which is what the argument scanner allocates
    // from, and no other thread exists to race the parse.
    unsafe { argv::setup() };

    app::init();

    // SAFETY: every step above has run, which is the state a static
    // constructor is entitled to find, and this runs once.
    unsafe { __libc_init_array() };
}

/// Closes what [`init`] opened and hands control back to the loader.
///
/// Corresponds to `__libnx_exit()` in `runtime/init.c`.
///
/// # Safety
///
/// Must be called at most once, on the thread that is exiting, with every
/// service [`init`] opened still open.
pub unsafe fn exit(_status: i32) -> ! {
    app::exit();

    // The status is discarded rather than forwarded, as it is upstream: libnx
    // passes a literal zero here too. Preserved so a loader that inspects the
    // value sees what it saw before.
    //
    // SAFETY: the function pointer is the loader's own, recorded while the
    // configuration block was parsed, and nothing runs after this.
    unsafe { __nx_exit(0, env::exit_func_ptr()) }
}

/// Initializes the reservation map that address-space lookups are served from.
///
/// Corresponds to `virtmemSetup()` in `virtmem.c`.
fn setup_virtmem() {
    #[cfg(feature = "sys-virtmem")]
    nx_sys_virtmem::virtmem::lock().init();

    #[cfg(not(feature = "sys-virtmem"))]
    {
        unsafe extern "C" {
            fn virtmemSetup();
        }

        // SAFETY: no other thread exists yet, which is the only thing this
        // step requires.
        unsafe { virtmemSetup() };
    }
}

/// Fills in the thread bookkeeping for the thread the loader started us on.
///
/// Corresponds to `__libnx_init_thread()` in `thread.c`. Runs after the heap
/// so the bookkeeping it registers has somewhere to live.
fn init_main_thread() {
    #[cfg(feature = "sys-thread")]
    {
        // SAFETY: this is the main thread, the heap is up, and no other thread
        // API has run yet.
        unsafe { nx_sys_thread::thread::init_main_thread() };
    }

    #[cfg(not(feature = "sys-thread"))]
    {
        unsafe extern "C" {
            fn __libnx_init_thread();
        }

        // SAFETY: as above.
        unsafe { __libnx_init_thread() };
    }
}

unsafe extern "C" {
    /// Runs the static constructors newlib collected into `.init_array`.
    ///
    /// newlib's own, and not overridden anywhere in this workspace.
    fn __libc_init_array();

    /// Restores the loader's stack pointer and branches back to it.
    ///
    /// Assembly, from whichever `.crt0` the link pipeline supplied: this
    /// crate's `crt0.s` under `rt-link`, libnx's `switch_crt0.s` otherwise.
    fn __nx_exit(status: u32, ret_addr: LoaderReturnFn) -> !;
}
