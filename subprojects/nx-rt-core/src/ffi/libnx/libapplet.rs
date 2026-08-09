//! Library applet launcher (`libapplet.c`) FFI.
//!
//! The surface a caller uses to run a library applet: build its common
//! arguments, exchange storages with it, start it, and read its reply. Every
//! function here reaches the applet through the runtime's applet singleton, the
//! same one [`crate::services::applet`] owns.
//!
//! It lives in `nx-rt-core` rather than an entry crate because none of it reads
//! an output-kind fact, so both kinds inherit one copy (`crates-rt`).
//!
//! # Why the whole surface is claimed
//!
//! libnx reaches the applet through `g_appletILibraryAppletCreator`, which is
//! `static` in `applet.c` and so cannot be aliased. The entry crates'
//! `appletInitialize` override replaces the only code that would populate it, so
//! any `libapplet*` function left to libnx dispatches against a zeroed session
//! rather than failing cleanly. Claiming every entry point is what keeps that
//! from happening, which is the same reason the `fs*` family is covered whole.
//!
//! # Where the behaviour comes from
//!
//! [`nx_service_applet::library_applet`] owns it; these are the C-facing
//! bindings onto it, and the mapping is one-to-one:
//!
//! | This module | `nx-service-applet` |
//! |---|---|
//! | [`__nx_rt_core__libnx_libapplet_launch`] | [`library_applet::launch`] |
//! | [`__nx_rt_core__libnx_libapplet_args_create`] | [`LibraryAppletArgs::new`] |
//! | [`__nx_rt_core__libnx_libapplet_args_set_play_startup_sound`] | `LibraryApplet::play_startup_sound` |
//! | [`__nx_rt_core__libnx_libapplet_create_write_storage`] | `LibraryAppletCreator::create_storage` + `Storage::write` |
//! | [`__nx_rt_core__libnx_libapplet_read_storage`] | `Storage::read` |
//! | [`__nx_rt_core__libnx_libapplet_push_in_data`] | `LibraryAppletAccessor::push_in_data` |
//! | [`__nx_rt_core__libnx_libapplet_pop_out_data`] | `LibraryAppletAccessor::pop_out_data` |
//! | [`__nx_rt_core__libnx_libapplet_start`] | `LibraryAppletAccessor::start` and `join` |
//!
//! The holder-taking commands need a bridge that does not exist yet: libnx's
//! `AppletHolder` and `AppletStorage` are C structs owning their own `Service`,
//! with no layout relationship to the Rust wrappers, so forwarding a caller's
//! holder means deciding how a C-owned holder maps onto a Rust-owned accessor.
//!
//! [`__nx_rt_core__libnx_libapplet_args_pop`], the jump flag and the
//! `libappletRequest*` family have no counterpart at all: they are
//! `ILibraryAppletSelfAccessor` commands, an applet addressing the system about
//! itself, and that sub-interface belongs in `nx-service-applet` beside the
//! creator and accessor.
//!
//! [`library_applet::launch`]: nx_service_applet::library_applet::launch
//! [`LibraryAppletArgs::new`]: nx_service_applet::LibraryAppletArgs::new

use core::ffi::c_void;

/// A user id, as libnx passes it by value.
///
/// Mirrors libnx's `AccountUid`. Declared here rather than taken from
/// `nx-service-acc` so this module does not carry a dependency for a type it
/// only forwards.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AccountUid {
    /// The two halves of the 128-bit id.
    pub uid: [u64; 2],
}

/// Initialises `args` for a library applet addressed with `version`.
///
/// # Safety
///
/// `args` must point to a writable `LibAppletArgs`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_libapplet_args_create(
    _args: *mut c_void,
    _version: u32,
) {
    todo!("libappletArgsCreate")
}

/// Sets whether the applet plays its startup sound.
///
/// # Safety
///
/// `args` must point to a writable `LibAppletArgs`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_libapplet_args_set_play_startup_sound(
    _args: *mut c_void,
    _flag: bool,
) {
    todo!("libappletArgsSetPlayStartupSound")
}

/// Creates a storage of `size` bytes holding `buffer`, and returns it in
/// `storage`.
///
/// # Safety
///
/// `storage` must point to a writable `AppletStorage`, and `buffer` to `size`
/// readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_libapplet_create_write_storage(
    _storage: *mut c_void,
    _buffer: *const c_void,
    _size: usize,
) -> u32 {
    todo!("libappletCreateWriteStorage")
}

/// Reads up to `size` bytes from offset 0 of `storage` into `buffer`.
///
/// A storage smaller than `size` is read whole, and `transfer_size` reports how
/// much was moved.
///
/// # Safety
///
/// `storage` must point to a readable `AppletStorage`, `buffer` to `size`
/// writable bytes, and `transfer_size` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_libapplet_read_storage(
    _storage: *mut c_void,
    _buffer: *mut c_void,
    _size: usize,
    _transfer_size: *mut usize,
) -> u32 {
    todo!("libappletReadStorage")
}

/// Stamps `args` with the current system tick and pushes it to `holder` as the
/// applet's first storage.
///
/// # Safety
///
/// `args` must point to a writable `LibAppletArgs` and `holder` to a writable
/// `AppletHolder`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_libapplet_args_push(
    _args: *mut c_void,
    _holder: *mut c_void,
) -> u32 {
    todo!("libappletArgsPush")
}

/// Reads the common arguments this applet was launched with into `args`.
///
/// Rejects arguments that do not validate.
///
/// # Safety
///
/// `args` must point to a writable `LibAppletArgs`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_libapplet_args_pop(_args: *mut c_void) -> u32 {
    todo!("libappletArgsPop")
}

/// Creates a storage holding `buffer` and pushes it to `holder`.
///
/// # Safety
///
/// `holder` must point to a writable `AppletHolder` and `buffer` to `size`
/// readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_libapplet_push_in_data(
    _holder: *mut c_void,
    _buffer: *const c_void,
    _size: usize,
) -> u32 {
    todo!("libappletPushInData")
}

/// Pops the applet's reply storage into `buffer` and closes it.
///
/// `transfer_size` reports how much was moved.
///
/// # Safety
///
/// `holder` must point to a writable `AppletHolder`, `buffer` to `size`
/// writable bytes, and `transfer_size` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_libapplet_pop_out_data(
    _holder: *mut c_void,
    _buffer: *mut c_void,
    _size: usize,
    _transfer_size: *mut usize,
) -> u32 {
    todo!("libappletPopOutData")
}

/// Sets whether a start jumps to the applet instead of running it and waiting.
///
/// Only an `AppletType_LibraryApplet` caller may set this.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_core__libnx_libapplet_set_jump_flag(_flag: bool) {
    todo!("libappletSetJumpFlag")
}

/// Starts the applet `holder` drives, waits for it to exit, and checks the exit
/// reason.
///
/// Jumps to the applet instead when the jump flag is set.
///
/// # Safety
///
/// `holder` must point to a writable `AppletHolder`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_libapplet_start(_holder: *mut c_void) -> u32 {
    todo!("libappletStart")
}

/// Runs library applet `applet_id` end to end: pushes `common_args` and `arg`,
/// starts it, waits, and reads its reply into `reply`.
///
/// `arg` and `reply` are optional, and `out_reply_size` reports how much of the
/// reply was moved.
///
/// # Safety
///
/// `common_args` must point to a readable `LibAppletArgs`, `arg` to `arg_size`
/// readable bytes or null, `reply` to `reply_size` writable bytes or null, and
/// `out_reply_size` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_libapplet_launch(
    _applet_id: u32,
    _common_args: *mut c_void,
    _arg: *const c_void,
    _arg_size: usize,
    _reply: *mut c_void,
    _reply_size: usize,
    _out_reply_size: *mut usize,
) -> u32 {
    todo!("libappletLaunch")
}

/// Returns to the HOME menu, as pressing the HOME button does.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_core__libnx_libapplet_request_home_menu() -> u32 {
    todo!("libappletRequestHomeMenu")
}

/// Enters the System Update flow, returning to the HOME menu on exit.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_core__libnx_libapplet_request_jump_to_system_update() -> u32 {
    todo!("libappletRequestJumpToSystemUpdate")
}

/// Asks the system to launch `application_id` for `uid`.
///
/// Available on 11.0.0+.
///
/// # Safety
///
/// `buffer` must point to `size` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_libapplet_request_to_launch_application(
    _application_id: u64,
    _uid: AccountUid,
    _buffer: *const c_void,
    _size: usize,
    _sender: u32,
) -> u32 {
    todo!("libappletRequestToLaunchApplication")
}

/// Asks the system to jump to the story flow for `uid`.
///
/// `application_id` is optional and may be zero. Available on 11.0.0+.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_core__libnx_libapplet_request_jump_to_story(
    _uid: AccountUid,
    _application_id: u64,
) -> u32 {
    todo!("libappletRequestJumpToStory")
}
