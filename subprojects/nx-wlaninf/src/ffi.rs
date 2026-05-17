//! C-FFI surface for the `wlan:inf` service.
//!
//! Exports `__nx_wlaninf__*` symbols that the `wlaninf_override.ld` linker
//! script aliases to libnx's `wlaninf.h` ABI (`wlaninfInitialize`,
//! `wlaninfExit`, `wlaninfGetServiceSession`, `wlaninfGetState`,
//! `wlaninfGetRSSI`). The shape mirrors `nx-pm`'s service-FFI pattern: lazy
//! SM connection, singleton state guarded by `RwLock`, and a
//! libnx-compatible `Service` shadow buffer returned by
//! `wlaninfGetServiceSession`.

use core::{cell::UnsafeCell, mem::MaybeUninit};

use nx_service_sm::SmService;
use nx_service_wlaninf::{DispatchError, GetRssiError, GetStateError, WlaninfService};
use nx_sf::{cmif, ffi::Service};
use nx_std_sync::rwlock::RwLock;
use nx_svc::error::ToRawResultCode;

/// Generic fallback used when no specific result code is available.
const GENERIC_ERROR: u32 = 0xFFFF;

/// `MAKERESULT(Module_Libnx, LibnxError_IncompatSysVer)` — returned when
/// the running firmware predates the wlan:inf service or it has been
/// retired (HOS 15.0.0+ removed the service entirely). Matches libnx's
/// `_wlaninfInitialize` gate.
const RC_INCOMPAT_SYSVER: u32 = 0x8A564;

/// Encodes a major/minor/micro firmware version the same way libnx's
/// `MAKEHOSVERSION` macro does.
#[inline]
const fn make_hos_version(major: u8, minor: u8, micro: u8) -> u32 {
    ((major as u32) << 16) | ((minor as u32) << 8) | (micro as u32)
}

unsafe extern "C" {
    /// Provided by libnx (or its `nx-rt` override) at link time.
    safe fn hosversionGet() -> u32;
}

/// Returns `true` when the running firmware is at least `major.minor.micro`.
#[inline]
fn hosversion_at_least(major: u8, minor: u8, micro: u8) -> bool {
    hosversionGet() >= make_hos_version(major, minor, micro)
}

/// `UnsafeCell` wrapper that asserts `Sync` for static storage. Access is
/// synchronised externally by the per-service `RwLock` below.
#[repr(transparent)]
struct SyncUnsafeCell<T>(UnsafeCell<T>);

// SAFETY: synchronisation is provided by the per-service singleton lock.
unsafe impl<T> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    fn get(&self) -> *mut T {
        self.0.get()
    }
}

/// Converts a CMIF [`cmif::ParseRespError`] to a raw libnx result code.
fn parse_resp_error_to_rc(err: cmif::ParseRespError) -> u32 {
    match err {
        cmif::ParseRespError::ServiceError(code) => code,
        cmif::ParseRespError::InvalidMagic
        | cmif::ParseRespError::Hipc(_)
        | cmif::ParseRespError::TruncatedOutHeader
        | cmif::ParseRespError::TruncatedDomainHeader
        | cmif::ParseRespError::TruncatedPayload
        | cmif::ParseRespError::TruncatedDomainObjects => GENERIC_ERROR,
    }
}

/// Converts a CMIF [`cmif::ParseRespBytesError`] to a raw libnx result code.
fn parse_resp_bytes_error_to_rc(err: cmif::ParseRespBytesError) -> u32 {
    match err {
        cmif::ParseRespBytesError::ServiceError(code) => code,
        cmif::ParseRespBytesError::InvalidMagic
        | cmif::ParseRespBytesError::Hipc(_)
        | cmif::ParseRespBytesError::TruncatedOutHeader
        | cmif::ParseRespBytesError::TruncatedDomainHeader
        | cmif::ParseRespBytesError::TruncatedPayload
        | cmif::ParseRespBytesError::TruncatedDomainObjects => GENERIC_ERROR,
    }
}

/// Converts a CMIF dispatch failure to its raw libnx result code.
fn dispatch_error_to_rc(err: DispatchError) -> u32 {
    match err {
        DispatchError::BuildRequest(_) => GENERIC_ERROR,
        DispatchError::SendRequest(e) => e.to_rc(),
        DispatchError::ParseResponse(e) => parse_resp_bytes_error_to_rc(e),
    }
}

/// Converts a `GetState` failure to its raw libnx result code.
fn get_state_error_to_rc(err: GetStateError) -> u32 {
    match err {
        GetStateError::Dispatch(d) => dispatch_error_to_rc(d),
        // The service returned a value outside the documented WlanInfState
        // range; map to a generic error since libnx forwards the raw u32
        // unconditionally and has no equivalent failure path.
        GetStateError::InvalidState(_) => GENERIC_ERROR,
    }
}

/// Converts a `GetRSSI` failure to its raw libnx result code.
fn get_rssi_error_to_rc(err: GetRssiError) -> u32 {
    dispatch_error_to_rc(err.0)
}

/// Converts an SM `GetService` failure to its raw libnx result code.
fn sm_get_service_error_to_rc(err: nx_service_sm::GetServiceCmifError) -> u32 {
    match err {
        nx_service_sm::GetServiceCmifError::BuildRequest(_) => GENERIC_ERROR,
        nx_service_sm::GetServiceCmifError::SendRequest(e) => e.to_rc(),
        nx_service_sm::GetServiceCmifError::ParseResponse(e) => parse_resp_error_to_rc(e),
        nx_service_sm::GetServiceCmifError::MissingHandle => GENERIC_ERROR,
    }
}

/// Converts an SM connect failure to its raw libnx result code.
fn sm_connect_error_to_rc(err: nx_service_sm::ConnectError) -> u32 {
    match err {
        nx_service_sm::ConnectError::Connect(e) => e.to_rc(),
        nx_service_sm::ConnectError::RegisterClient(e) => match e {
            nx_service_sm::RegisterClientCmifError::BuildRequest(_) => GENERIC_ERROR,
            nx_service_sm::RegisterClientCmifError::SendRequest(e) => e.to_rc(),
            nx_service_sm::RegisterClientCmifError::ParseResponse(e) => parse_resp_error_to_rc(e),
        },
    }
}

/// Private SM session opened on demand by the wlaninf FFI. Never torn down.
static SM: RwLock<Option<SmService>> = RwLock::new(None);

/// Cached `wlan:inf` session.
static WLANINF: RwLock<Option<WlaninfService>> = RwLock::new(None);

/// `Service` shadow buffer backing `wlaninfGetServiceSession`.
static WLANINF_SRV: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// Acquires or returns the cached SM session.
fn ensure_sm() -> Result<(), nx_service_sm::ConnectError> {
    {
        let guard = SM.read();
        if guard.is_some() {
            return Ok(());
        }
    }

    let mut guard = SM.write();
    if guard.is_some() {
        return Ok(());
    }
    let sm = nx_service_sm::connect()?;
    *guard = Some(sm);
    Ok(())
}

/// Populates the `Service` shadow buffer with a non-owning view of `session`.
///
/// `own_handle = 0`, `object_id = 0` (libnx's "override" mode) — the Rust
/// singleton retains exclusive ownership of the kernel handle. The shadow
/// buffer's `Service` must not call `serviceClose` on the cached pointer.
fn write_shadow(session: &nx_sf::service::Session) {
    let service = Service {
        session: session.handle(),
        own_handle: 0,
        object_id: 0,
        pointer_buffer_size: session.pointer_buffer_size(),
    };
    // SAFETY: called while holding the WLANINF write lock, so no other
    // thread is reading the shadow buffer.
    unsafe { WLANINF_SRV.get().cast::<Service>().write(service) };
}

/// Zeroes the `Service` shadow buffer on exit so a stray reader sees an
/// `INVALID_HANDLE` rather than a freed kernel handle.
fn clear_shadow() {
    // SAFETY: called while holding the WLANINF write lock.
    unsafe { WLANINF_SRV.get().write(MaybeUninit::zeroed()) };
}

/// `wlaninfInitialize()` — opens a session to `wlan:inf`.
///
/// Returns `RC_INCOMPAT_SYSVER` on HOS 15.0.0+ to match libnx, which refuses
/// the connection because the service has been retired.
///
/// # Safety
///
/// Callable from any thread. Must be paired with [`__nx_wlaninf__wlaninf_exit`]
/// before the program exits. The hosversion query is provided by libnx /
/// `nx-rt` at link time; calling this before that runtime is initialised is
/// undefined behaviour.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_wlaninf__wlaninf_initialize() -> u32 {
    if hosversion_at_least(15, 0, 0) {
        return RC_INCOMPAT_SYSVER;
    }

    {
        let guard = WLANINF.read();
        if guard.is_some() {
            return 0;
        }
    }

    if let Err(err) = ensure_sm() {
        return sm_connect_error_to_rc(err);
    }

    let sm_guard = SM.read();
    let sm = sm_guard.as_ref().expect("SM not initialized");

    let svc = match nx_service_wlaninf::connect_cmif(sm) {
        Ok(s) => s,
        Err(e) => return sm_get_service_error_to_rc(e.0),
    };

    let mut guard = WLANINF.write();
    if guard.is_some() {
        // Lost the race; drop the duplicate session.
        return 0;
    }
    write_shadow(svc.session());
    *guard = Some(svc);
    0
}

/// `wlaninfExit()` — closes the `wlan:inf` session.
///
/// # Safety
///
/// Callable from any thread. Calling concurrently with the other
/// `__nx_wlaninf__*` symbols is safe (singleton state is `RwLock`-guarded),
/// but invalidates any `Service*` previously returned by
/// [`__nx_wlaninf__wlaninf_get_service_session`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_wlaninf__wlaninf_exit() {
    let mut guard = WLANINF.write();
    if guard.take().is_some() {
        clear_shadow();
    }
}

/// `wlaninfGetServiceSession()` — returns the libnx `Service*` for
/// `wlan:inf`.
///
/// # Safety
///
/// The returned pointer is valid until [`__nx_wlaninf__wlaninf_exit`] is
/// called. The shadow buffer holds a non-owning view of the Rust-managed
/// session; the caller must not invoke `serviceClose` on it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_wlaninf__wlaninf_get_service_session() -> *mut Service {
    WLANINF_SRV.get().cast::<Service>()
}

/// `wlaninfGetState(WlanInfState *out)`.
///
/// # Safety
///
/// `out` must be either null or a valid, writable pointer to a `u32` for
/// the duration of the call. A null pointer returns a generic error
/// without dispatching the IPC.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_wlaninf__wlaninf_get_state(out: *mut u32) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let guard = WLANINF.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.get_state() {
        Ok(state) => {
            // `WlanInfState` is `#[repr(u32)]`, sized to match libnx's
            // `WlanInfState` enum.
            // SAFETY: caller guarantees `out` is writable for `u32`.
            unsafe { *out = state as u32 };
            0
        }
        Err(e) => get_state_error_to_rc(e),
    }
}

/// `wlaninfGetRSSI(s32 *out)`.
///
/// # Safety
///
/// `out` must be either null or a valid, writable pointer to an `i32` for
/// the duration of the call. A null pointer returns a generic error
/// without dispatching the IPC.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_wlaninf__wlaninf_get_rssi(out: *mut i32) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let guard = WLANINF.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.get_rssi() {
        Ok(rssi) => {
            // SAFETY: caller guarantees `out` is writable for `i32`.
            unsafe { *out = rssi.dbm() };
            0
        }
        Err(e) => get_rssi_error_to_rc(e),
    }
}
