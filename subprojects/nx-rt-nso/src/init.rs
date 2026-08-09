//! The startup sequence the `.crt0` hands off to, and the way back out.
//!
//! Ports libnx's `__libnx_init` and `__libnx_exit` for a `pm`-launched NSO:
//! the outermost pair of the C runtime, sitting between the assembly that
//! self-relocates the image and the program's `main`. [`init`] is the whole of
//! startup, from the environment state through to the static constructors;
//! [`exit`] is the return path out of the process. The steps themselves belong
//! elsewhere, most of them to other crates; what lives here is the order they
//! run in.
//!
//! # How this differs from the homebrew-NRO sequence
//!
//! Upstream keeps one `__libnx_init` for every output kind and forks inside
//! the steps: `envSetup` treats a null configuration block as "this is an
//! NSO", and `argvSetup` reads `__argdata__` rather than a loader string. Here
//! the fork is gone, because the crate a step lives in has already answered
//! the question. Two consequences are visible in the order below.
//!
//! The environment step takes no configuration block. An NSO is handed none,
//! so [`crate::env::setup`] has nothing to parse and does not accept the
//! argument at all; the `.crt0` still passes a zero in that register, and it
//! is ignored here rather than tested.
//!
//! The service bring-up is still libnx's. `__appInit` is not overridden for
//! this output kind: the sequence reaches it through its C name, and the
//! per-service overrides inside it redirect individually. That is also why
//! nothing here mounts the SD card or sets a working directory: those are
//! steps inside `__appInit`, and upstream skips both for an NSO anyway.
//!
//! # A step this build did not take over
//!
//! Two of the steps live in crates this one depends on only optionally, behind
//! the `sys-virtmem` and `sys-thread` features, so a build with the matching
//! Meson option off reaches them with the crate absent. Such a step calls
//! libnx's entry point by its C name rather than skipping: skipping would
//! leave the work undone, and with the feature off that C name can only be
//! libnx's own implementation, because the linker fragment that would redirect
//! it is added by the same Meson option that sets the feature.
//!
//! `__libc_init_array` is not one of those. It is newlib's, not libnx's, and
//! nothing in this workspace replaces it, so it is always reached through C.

use nx_svc::thread::Handle as ThreadHandle;

use crate::{
    argv,
    env::{
        self,
        LoaderReturnFn,
    },
};

/// Brings the process up to the point `main` can be entered.
///
/// Corresponds to `__libnx_init()` in `runtime/init.c`.
///
/// # Safety
///
/// Must be called exactly once, on the startup thread, with the arguments the
/// process manager's launch handed the `.crt0`: `main_thread` the handle of
/// the process main thread, and `saved_lr` the address to return through.
pub unsafe fn init(main_thread: ThreadHandle, saved_lr: LoaderReturnFn) {
    env::setup(main_thread, saved_lr);

    // SAFETY: the environment is populated, so the main thread's handle is
    // known, and no thread-local has been touched yet.
    unsafe { env::main_thread::setup() };

    setup_virtmem();
    nx_rt_core::init::setup_heap();
    init_main_thread();

    // SAFETY: the heap is up, which is what the argument scanner allocates
    // from, and no other thread exists to race the parse.
    unsafe { argv::setup() };

    // SAFETY: the runtime is up, which is the state the service bring-up
    // expects, and it runs once.
    unsafe { __appInit() };

    // SAFETY: every step above has run, which is the state a static
    // constructor is entitled to find, and this runs once.
    unsafe { __libc_init_array() };
}

/// Closes what [`init`] opened and leaves the process.
///
/// Corresponds to `__libnx_exit()` in `runtime/init.c`.
///
/// # Safety
///
/// Must be called at most once, on the thread that is exiting, with every
/// service [`init`] opened still open.
pub unsafe fn exit(_status: i32) -> ! {
    // SAFETY: teardown of the services the sequence above opened, on the
    // exiting thread.
    unsafe { __appExit() };

    // The status is discarded rather than forwarded, as it is upstream: libnx
    // passes a literal zero here too. Preserved so whatever reads the value
    // sees what it saw before.
    //
    // SAFETY: the function pointer is the one recorded during environment
    // bring-up, and nothing runs after this.
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

/// Fills in the thread bookkeeping for the thread the process started on.
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
    /// Opens the default services, and runs the program's `userAppInit` hook.
    ///
    /// libnx's own: this output kind does not override `__appInit`, so the
    /// whole sequence stays in C and the per-service overrides inside it
    /// redirect one at a time.
    fn __appInit();

    /// Closes what `__appInit` opened, and runs `userAppExit` first.
    ///
    /// libnx's own, for the reason given on `__appInit`.
    fn __appExit();

    /// Runs the static constructors newlib collected into `.init_array`.
    ///
    /// newlib's own, and not overridden anywhere in this workspace.
    fn __libc_init_array();

    /// Restores the launch stack pointer and branches to the return address.
    ///
    /// Assembly, from whichever `.crt0` the link pipeline supplied: this
    /// crate's `crt0.s` under `rt-link`, libnx's `switch_crt0.s` otherwise.
    fn __nx_exit(status: u32, ret_addr: LoaderReturnFn) -> !;
}
