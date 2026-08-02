//! System Settings (set:sys) service FFI

use core::mem::MaybeUninit;

use nx_rt_core::error::ToResultCode as _;
use nx_sf::{error::ToResultCode, ffi::Service};

use crate::{
    ffi::common::{GENERIC_ERROR, SyncUnsafeCell},
    services::set,
};

/// Static buffer for set:sys FFI session access. Updated on `initialize()` and `exit()`.
static SETSYS_FFI_SESSION: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// Initializes set:sys connection. Returns 0 on success, error code on failure.
///
/// Corresponds to `setsysInitialize()` in `set.h`.
///
/// # Safety
///
/// SM must be initialized before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_setsys_initialize() -> u32 {
    if let Err(err) = set::init() {
        return err.to_rc();
    }
    if let Some(setsys) = set::get_service() {
        // Non-owning FFI view (`own_handle = 0`, `object_id = 0` — libnx's
        // "override" mode): the Rust `SetSysService` retains exclusive
        // ownership and closes the kernel handle on `Drop` via `set::exit`.
        // A `own_handle = 1` snapshot would risk a double-close if libnx
        // ever invoked `serviceClose` on the cached pointer.
        let service = Service {
            session: setsys.session(),
            own_handle: 0,
            object_id: 0,
            pointer_buffer_size: 0,
        };
        // SAFETY: Called only during initialization; no concurrent readers.
        unsafe { SETSYS_FFI_SESSION.get().cast::<Service>().write(service) };
    }
    0
}

/// Closes set:sys connection.
///
/// Corresponds to `setsysExit()` in `set.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_setsys_exit() {
    set::exit();
    // SAFETY: Called only during exit, after the service has been closed.
    unsafe { SETSYS_FFI_SESSION.get().write(MaybeUninit::zeroed()) };
}

/// Gets the set:sys service session pointer.
///
/// Corresponds to `setsysGetServiceSession()` in `set.h`.
///
/// # Safety
///
/// Returns a pointer to the service session or null if not initialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_setsys_get_service_session() -> *mut Service {
    SETSYS_FFI_SESSION.get().cast::<Service>()
}

/// Gets the system firmware version. Returns 0 on success, error code on failure.
///
/// Corresponds to `setsysGetFirmwareVersion()` in `set.h`.
///
/// # Safety
///
/// `out` must point to valid, writable memory for a FirmwareVersion struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_setsys_get_firmware_version(
    out: *mut nx_service_set::FirmwareVersion,
) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let Some(setsys) = set::get_service() else {
        return GENERIC_ERROR;
    };

    let fw = match setsys.get_firmware_version_cmif() {
        Ok(fw) => fw,
        Err(err) => return err.to_rc(),
    };

    // SAFETY: Caller guarantees out points to valid memory.
    unsafe { *out = fw };
    0
}
