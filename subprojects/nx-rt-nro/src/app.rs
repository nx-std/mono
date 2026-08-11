//! Service bring-up and teardown around `main`.
//!
//! Ports libnx's `__appInit` and `__appExit`: the pair the runtime entry point
//! wraps around a homebrew program's `main`, opening the services such a
//! program expects to find already connected and closing them again on the way
//! out. The steps themselves belong to the per-service managers under
//! [`crate::services`]; what lives here is the order they run in.
//!
//! # Why the order lives in this crate
//!
//! Most of the sequence is kind-agnostic, but its tail is not: mounting the SD
//! card and changing into the directory the program was loaded from only mean
//! something for an executable the loader launched by path. The version
//! resolution step is anchored here too, because the `set:sys` session it
//! borrows is one of this crate's per-service managers. Splitting the sequence
//! so its middle lived in [`nx_rt_core`] would leave neither half readable as
//! the startup order it encodes, so the whole of it stays here until a second
//! output kind needs one of the same steps.
//!
//! # A service this build did not take over
//!
//! Every step but the Service Manager is behind a `service-*` Cargo feature,
//! and each is off unless its `use_nx_service_*` Meson option turns it on, so
//! a default build reaches this sequence with most of them absent. A step
//! whose feature is off therefore calls libnx's own entry point by its C name
//! rather than skipping: skipping would leave the service unopened, which is
//! not what the C `__appInit` this replaces did in the same configuration.
//!
//! Reaching for the C name is safe precisely because the feature is off. The
//! linker fragment that would redirect that name to this workspace is added by
//! the same Meson option that sets the feature, so with the feature off the
//! name can only be libnx's own implementation, and the Rust arm is compiled
//! out. With the feature on the Rust arm calls the manager directly and the
//! declaration is gone.
//!
//! # Failure
//!
//! libnx aborts through `diagAbortWithResult`, with a `Module_Libnx` code
//! naming the step that failed. Nothing in this workspace binds that function,
//! and the fatal path here is a panic: `nx-panic-handler` breaks with the
//! panic message, which names both the step and the error under it rather than
//! a bare code. The set of steps treated as fatal is unchanged.

use nx_rt_core::env::hos_version::{
    self,
    HosVersion,
};

use crate::{
    cwd,
    services::sm,
};

/// Opens the services a homebrew program expects `main` to find connected.
///
/// Corresponds to `__appInit()` in `runtime/init.c`.
///
/// # Panics
///
/// Panics when a service libnx treats as mandatory fails to open: the Service
/// Manager, the Applet Manager, HID, Time or the filesystem. Version
/// resolution and the SD card mount are not in that set; see the step
/// functions for why each is allowed to fail.
pub fn init() {
    init_sm();
    resolve_hos_version();
    init_applet();
    init_hid();
    init_time();
    init_fs();
    mount_sdmc();
    cwd::init();

    // SAFETY: both hooks run on the startup thread with every service above
    // already open, which is the state libnx calls them in.
    unsafe {
        win_init();
        user_app_init();
    }
}

/// Closes what [`init`] opened, in the reverse order.
///
/// Corresponds to `__appExit()` in `runtime/init.c`.
pub fn exit() {
    // SAFETY: both hooks run on the exiting thread with every service still
    // open, which is the state libnx calls them in.
    unsafe {
        user_app_exit();
        win_exit();
    }

    unmount_all();
    exit_fs();
    exit_time();
    exit_hid();
    exit_applet();
    exit_sm();
}

/// Bootstraps the Service Manager, which every other service is reached through.
///
/// Never has a libnx arm: `nx-rt-core` is not optional, so this crate always
/// has the Rust Service Manager to call.
fn init_sm() {
    if let Err(err) = sm::initialize() {
        panic!("startup: the Service Manager failed to initialize: {err}");
    }
}

/// Closes the Service Manager session. See [`init_sm`] for why it has one arm.
fn exit_sm() {
    sm::exit();
}

/// Publishes the Horizon OS version, which the protocol choices below it read.
///
/// Only queried when nothing has published a version yet: the homebrew loader
/// may already have supplied one through its configuration block, and the
/// query costs a `set:sys` session that is closed again straight away.
///
/// A failure is not fatal, here or in libnx: the version stays unset and the
/// callers that read it fall back to their pre-3.0.0 behaviour, which is the
/// same thing that happens on a console old enough to have no answer to give.
fn resolve_hos_version() {
    // libnx tests the raw global, which carries the Atmosphere flag in its top
    // bit, so a run that recorded only that flag already counts as set.
    // `hos_version::get` masks the flag off, so it is tested on its own.
    if hos_version::is_atmosphere() || hos_version::get() != HosVersion::default() {
        return;
    }

    cfg_select! {
        feature = "service-set" => {
            use crate::services::set;

            // Every failure below leaves the version unset, which is the outcome
            // the doc comment describes, so none of them is reported.
            if set::init().is_err() {
                return;
            }
            if let Ok(fw) = set::firmware_version() {
                hos_version::set(HosVersion::new(fw.major, fw.minor, fw.patch).as_u32());
            }
            set::exit();
        }
        _ => {
            // The layout libnx writes into, from `SetSysFirmwareVersion` in
            // `services/set.h`. Only the three version bytes are read; the rest is
            // carried so the buffer is the 0x100 bytes the command fills.
            #[repr(C)]
            struct FirmwareVersion {
                major: u8,
                minor: u8,
                micro: u8,
                rest: [u8; 0xFD],
            }

            unsafe extern "C" {
                fn setsysInitialize() -> u32;
                fn setsysGetFirmwareVersion(out: *mut FirmwareVersion) -> u32;
                fn setsysExit();
            }

            // SAFETY: the Service Manager is open, which is all `setsysInitialize`
            // requires, and the out-pointer below addresses a live local.
            unsafe {
                if setsysInitialize() != 0 {
                    return;
                }

                let mut fw = FirmwareVersion {
                    major: 0,
                    minor: 0,
                    micro: 0,
                    rest: [0; 0xFD],
                };
                if setsysGetFirmwareVersion(&raw mut fw) == 0 {
                    hos_version::set(HosVersion::new(fw.major, fw.minor, fw.micro).as_u32());
                }

                setsysExit();
            }
        }
    }
}

/// Opens the Applet Manager session and performs the per-role handshake.
fn init_applet() {
    cfg_select! {
        feature = "service-applet" => {
            if let Err(err) = crate::services::applet::init_from_env() {
                panic!("startup: the Applet Manager failed to initialize: {err}");
            }
        }
        _ => {
            unsafe extern "C" {
                fn appletInitialize() -> u32;
            }

            // SAFETY: the Service Manager is open, which is what this requires.
            let rc = unsafe { appletInitialize() };
            if rc != 0 {
                panic!("startup: the Applet Manager failed to initialize: {rc:#x}");
            }
        }
    }
}

/// Closes the Applet Manager session.
fn exit_applet() {
    cfg_select! {
        feature = "service-applet" => {
            crate::services::applet::exit();
        }
        _ => {
            unsafe extern "C" {
                fn appletExit();
            }

            // SAFETY: teardown of a session this sequence opened.
            unsafe { appletExit() };
        }
    }
}

/// Opens the HID session and maps the shared memory the pads are read from.
///
/// Upstream guards this on the applet type not being `None`, because one
/// translation unit serves every output kind there. Here the guard is gone:
/// the homebrew loader sources this crate's applet type, and its configuration
/// block has no encoding for `None`: [`crate::env::applet_type`] cannot
/// return one, so the branch could only ever go one way.
fn init_hid() {
    cfg_select! {
        feature = "service-hid" => {
            if let Err(err) = crate::services::hid::init() {
                panic!("startup: HID failed to initialize: {err}");
            }
        }
        _ => {
            unsafe extern "C" {
                fn hidInitialize() -> u32;
            }

            // SAFETY: the Applet Manager is open, which HID reads the applet
            // resource user id from.
            let rc = unsafe { hidInitialize() };
            if rc != 0 {
                panic!("startup: HID failed to initialize: {rc:#x}");
            }
        }
    }
}

/// Closes the HID session.
///
/// libnx calls this unconditionally even though it opened HID behind a guard,
/// and that stays true here: closing a session that was never opened is a
/// no-op, so the two do not need to agree.
fn exit_hid() {
    cfg_select! {
        feature = "service-hid" => {
            crate::services::hid::exit();
        }
        _ => {
            unsafe extern "C" {
                fn hidExit();
            }

            // SAFETY: teardown of a session this sequence opened.
            unsafe { hidExit() };
        }
    }
}

/// Opens the Time session and anchors the realtime clock from it.
///
/// The anchoring step is libnx's `__libnx_init_time`, and its failure is not
/// fatal in libnx either: the clock is left unanchored and the timezone unset,
/// which costs a program that reads the wall clock and nothing else.
fn init_time() {
    cfg_select! {
        feature = "service-time" => {
            use crate::services::time;

            if let Err(err) = time::init() {
                panic!("startup: Time failed to initialize: {err}");
            }
            // A clock that could not be anchored is the documented outcome of this
            // failing, and there is no caller to report it to this early.
            let _ = time::init_wall_clock();
        }
        _ => {
            unsafe extern "C" {
                fn timeInitialize() -> u32;
                fn __libnx_init_time();
            }

            // SAFETY: the Service Manager is open, which is what these require.
            unsafe {
                let rc = timeInitialize();
                if rc != 0 {
                    panic!("startup: Time failed to initialize: {rc:#x}");
                }
                __libnx_init_time();
            }
        }
    }
}

/// Closes the Time session.
fn exit_time() {
    cfg_select! {
        feature = "service-time" => {
            crate::services::time::exit();
        }
        _ => {
            unsafe extern "C" {
                fn timeExit();
            }

            // SAFETY: teardown of a session this sequence opened.
            unsafe { timeExit() };
        }
    }
}

/// Opens the `fsp-srv` session every mount and every file is addressed inside.
fn init_fs() {
    cfg_select! {
        feature = "service-fs" => {
            if let Err(err) = crate::services::fs::init() {
                panic!("startup: the filesystem failed to initialize: {err}");
            }
        }
        _ => {
            unsafe extern "C" {
                fn fsInitialize() -> u32;
            }

            // SAFETY: the Service Manager is open, which is what this requires.
            let rc = unsafe { fsInitialize() };
            if rc != 0 {
                panic!("startup: the filesystem failed to initialize: {rc:#x}");
            }
        }
    }
}

/// Closes the `fsp-srv` session, after every mount inside it is gone.
fn exit_fs() {
    cfg_select! {
        feature = "service-fs" => {
            crate::services::fs::exit();
        }
        _ => {
            unsafe extern "C" {
                fn fsExit();
            }

            // SAFETY: teardown of a session this sequence opened, and every mount
            // inside it was unmounted first.
            unsafe { fsExit() };
        }
    }
}

/// Mounts the SD card as `sdmc:`, which is where a homebrew program's files are.
///
/// libnx ignores the result, and so does this: a console with no SD card
/// inserted still runs a program that never touches one, and the mount failing
/// is reported again by the first path that tries to resolve through it.
fn mount_sdmc() {
    cfg_select! {
        feature = "service-fs" => {
            // Reporting this would abort a program that never touches the SD card, on a
            // console where none is inserted. The failure is not lost: the first path that
            // resolves through `sdmc:` reports it again, to a caller that can act on it.
            let _ = nx_fsdev::mount::mount_sdmc();
        }
        _ => {
            unsafe extern "C" {
                fn fsdevMountSdmc() -> u32;
            }

            // Discarded for the reason above.
            // SAFETY: the `fsp-srv` session is open, which is what this requires.
            let _ = unsafe { fsdevMountSdmc() };
        }
    }
}

/// Unmounts every filesystem, so no mount outlives the session holding it.
fn unmount_all() {
    cfg_select! {
        feature = "service-fs" => {
            nx_fsdev::mount::unmount_all();
        }
        _ => {
            unsafe extern "C" {
                fn fsdevUnmountAll() -> u32;
            }

            // A mount that failed to come down is one this call wanted gone anyway, and the
            // process is exiting, so there is no caller left to report it to. The Rust arm
            // above returns nothing for the same reason.
            // SAFETY: teardown of the mounts this sequence made.
            let _ = unsafe { fsdevUnmountAll() };
        }
    }
}

// The four hooks below are the program's, not libnx's. Each is declared weak
// and undefined, so a program that does not define one leaves the symbol null
// and the call is skipped: that is how libnx's `if (&hook) hook();` reads, and
// `extern_weak` is what expresses it here. A plain declaration would instead
// fail the link for every program that does not supply all four.
//
// `__nx_win_init` and `__nx_win_exit` are libnx's own, defined in
// `display/default_window.c` and pulled into the link only by a program that
// reaches for the default window. `userAppInit` and `userAppExit` are never
// defined by libnx at all.

unsafe extern "C" {
    #[linkage = "extern_weak"]
    static __nx_win_init: Option<unsafe extern "C" fn()>;
    #[linkage = "extern_weak"]
    static __nx_win_exit: Option<unsafe extern "C" fn()>;
    #[linkage = "extern_weak"]
    static userAppInit: Option<unsafe extern "C" fn()>;
    #[linkage = "extern_weak"]
    static userAppExit: Option<unsafe extern "C" fn()>;
}

/// Sets up the default window, when the program linked one in.
///
/// # Safety
///
/// The services the default window is built on must already be open.
unsafe fn win_init() {
    // SAFETY: reading the weak symbol yields the hook, or `None` when the
    // program defined none. The value is read before any reference is taken,
    // so an absent symbol is never dereferenced.
    if let Some(hook) = unsafe { __nx_win_init } {
        // SAFETY: the caller guarantees the state the hook expects.
        unsafe { hook() };
    }
}

/// Tears down the default window. See [`win_init`].
///
/// # Safety
///
/// The services the default window is built on must still be open.
unsafe fn win_exit() {
    // SAFETY: as in `win_init`.
    if let Some(hook) = unsafe { __nx_win_exit } {
        // SAFETY: the caller guarantees the state the hook expects.
        unsafe { hook() };
    }
}

/// Runs the program's own startup hook, when it defined one.
///
/// # Safety
///
/// Every service this sequence opens must already be open.
unsafe fn user_app_init() {
    // SAFETY: as in `win_init`.
    if let Some(hook) = unsafe { userAppInit } {
        // SAFETY: the caller guarantees the state the hook expects.
        unsafe { hook() };
    }
}

/// Runs the program's own teardown hook. See [`user_app_init`].
///
/// # Safety
///
/// Every service this sequence opens must still be open.
unsafe fn user_app_exit() {
    // SAFETY: as in `win_init`.
    if let Some(hook) = unsafe { userAppExit } {
        // SAFETY: the caller guarantees the state the hook expects.
        unsafe { hook() };
    }
}
