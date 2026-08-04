//! Service lifecycle commands.

use core::mem::MaybeUninit;

use nx_rt_core::error::ToResultCode as _;
use nx_service_fs::Priority;
use nx_sf::ffi::Service;

use crate::{
    ffi::common::SyncUnsafeCell,
    services::fs,
};

/// Backing storage for [`__nx_rt_nro__libnx_fs_get_service_session`], which
/// hands C a pointer rather than a value. Written on `fsInitialize` and zeroed
/// on `fsExit`.
static FS_FFI_SESSION: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::zeroed());

/// Initializes the `fsp-srv` service.
///
/// Corresponds to `fsInitialize()` in libnx.
///
/// # Safety
///
/// SM must be initialized before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_initialize() -> u32 {
    if let Err(err) = fs::init() {
        return err.to_rc();
    }

    if let Some(service) = fs::get_service() {
        // Domain root, in libnx's `Service` encoding: a non-zero `object_id`
        // with `own_handle` left at zero, so the view describes the domain
        // without claiming the close.
        let view = Service {
            session: service.session_handle().to_handle(),
            own_handle: 0,
            object_id: service.root_object_id().to_raw(),
            pointer_buffer_size: 0,
        };
        // SAFETY: Called only during initialization; no concurrent readers.
        unsafe { FS_FFI_SESSION.get().cast::<Service>().write(view) };
    }

    0
}

/// Closes the `fsp-srv` service.
///
/// Corresponds to `fsExit()` in libnx.
///
/// # Safety
///
/// No filesystem, file or directory this module handed out may still be in use:
/// closing the session invalidates every object id issued within it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_exit() {
    fs::exit();
    // SAFETY: Called only during exit, after the session has been closed.
    unsafe { FS_FFI_SESSION.get().write(MaybeUninit::zeroed()) };
}

/// Returns the `fsp-srv` service session.
///
/// Corresponds to `fsGetServiceSession()` in libnx, which hands back its
/// `g_fsSrv`. The view describes the same thing this crate owns: a domain whose
/// root is `fsp-srv` itself, carrying the object id the conversion assigned it.
///
/// `own_handle` is zero because the Rust `FsService` keeps the close: a C caller
/// that ran `serviceClose` on an owning snapshot would tear down the session out
/// from under the pool.
///
/// # Safety
///
/// The returned pointer is valid until `fsExit`, and the `Service` it addresses
/// must not be closed by the caller.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_get_service_session() -> *mut Service {
    FS_FFI_SESSION.get().cast::<Service>()
}

/// Sets the request priority applied to subsequent `fsp-srv` commands.
///
/// Corresponds to `fsSetPriority()` in libnx, which ignores the request before
/// HOS 5.0.0. The priority rides in the CMIF context word, which older servers
/// do not read, so applying it unconditionally is the same no-op without the
/// version query.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety; an unrecognized priority
/// is ignored.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_set_priority(priority: u32) {
    let priority = match priority {
        0 => Priority::Normal,
        1 => Priority::Realtime,
        2 => Priority::Low,
        3 => Priority::Background,
        _ => return,
    };

    if let Some(service) = fs::get_service() {
        service.set_priority(priority);
    }
}
