//! # Runtime Environment State
//!
//! Kind-agnostic container for the runtime environment every Switch
//! executable shares: the main-thread and own-process handles, the heap
//! override region, `argv`, syscall-availability hints, the applet type,
//! loader-supplied service overrides, the random seed, and the chain-loading
//! buffers.
//!
//! Each output kind parses its own startup source — the homebrew loader
//! configuration for an NRO, a build-time profile for an NSO — and fills this
//! container exactly once through [`init_once`]. Every other consumer reads it
//! back through the accessor functions below, which are sound precisely
//! because the state is written once and immutable thereafter.

pub mod hos_version;
pub mod main_thread;
mod syscall_hint;

use core::{
    cell::UnsafeCell,
    ffi::{c_char, c_void},
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, Ordering},
};

pub use nx_sf::ServiceName;
use nx_svc::{
    ipc::Handle as ServiceHandle, process::Handle as ProcessHandle, thread::Handle as ThreadHandle,
};
use nx_sys_sync::{Mutex, Once};

pub use self::syscall_hint::SyscallHints;

/// Loader return function type
pub type LoaderReturnFn = Option<unsafe extern "C" fn(i32) -> !>;

/// Maximum number of service overrides (matches libnx MAX_OVERRIDES)
pub const MAX_SERVICE_OVERRIDES: usize = 32;

/// Global environment state (immutable after initialization)
static ENV_STATE: EnvStateWrapper = EnvStateWrapper::new();

/// Initialization guard to ensure the env state is populated exactly once
static ENV_INIT: Once = Once::new();

/// Exit function pointer (mutable at runtime)
static EXIT_FUNC: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

static NEXT_LOAD: NextLoadState = NextLoadState::new();

/// Populate the global environment state exactly once.
///
/// The `populate` closure receives exclusive `&mut` access to the [`EnvState`]
/// container and is responsible for filling it from whatever startup source
/// the calling entry crate owns — the homebrew loader configuration for an
/// NRO, a build-time profile for an NSO, and so on.
///
/// Subsequent calls are no-ops: the state is written once here and is
/// read-only afterwards, which is what makes the unsynchronized accessor
/// functions below sound.
pub fn init_once(populate: impl FnOnce(&mut EnvState)) {
    ENV_INIT.call_once(|| {
        // SAFETY: `Once::call_once` guarantees this runs exactly once with
        // exclusive access; no accessor can observe the state mid-write.
        let state = unsafe { &mut *ENV_STATE.get() };
        populate(state);
    });
}

/// Get loader info string pointer and size
pub fn loader_info() -> Option<(NonNull<c_char>, u64)> {
    // SAFETY: ENV_STATE is initialized once via init_once() before any other
    // function is called. After initialization, the state is read-only.
    let state = unsafe { ENV_STATE.get_ref() };
    state.loader_info
}

/// Get main thread handle
///
/// # Panics
///
/// Panics if called before the environment is initialized.
pub fn main_thread_handle() -> ThreadHandle {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state
        .main_thread_handle
        .expect("main thread handle not set")
}

/// Returns true if running as NSO, false if NRO
pub fn is_nso() -> bool {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.is_nso
}

/// Get heap override address and size if present
///
/// Returns `Some((addr, size))` if the homebrew loader provided a heap override,
/// or `None` if running without a heap override.
pub fn heap_override() -> Option<(NonNull<c_void>, usize)> {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.heap_override
}

/// Get argv string pointer if present
pub fn argv() -> Option<*const c_char> {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.argv.map(|ptr| ptr.as_ptr() as *const c_char)
}

/// Get syscall availability hints
///
/// # Panics
///
/// Panics if called before the environment is initialized.
pub fn syscall_hints() -> SyscallHints {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.syscall_hints.expect("syscall hints not set")
}

/// Get process handle if present
pub fn own_process_handle() -> Option<ProcessHandle> {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.process_handle
}

/// Set exit function pointer
pub fn set_exit_func_ptr(func: LoaderReturnFn) {
    let ptr = match func {
        None => ptr::null_mut(),
        Some(f) => f as *mut c_void,
    };
    EXIT_FUNC.store(ptr, Ordering::Release);
}

/// Get exit function pointer
pub fn exit_func_ptr() -> LoaderReturnFn {
    let ptr = EXIT_FUNC.load(Ordering::Acquire);
    if ptr.is_null() {
        None
    } else {
        // SAFETY: The pointer was stored via set_exit_func_ptr which ensures validity
        Some(unsafe {
            core::mem::transmute::<*mut core::ffi::c_void, unsafe extern "C" fn(i32) -> !>(ptr)
        })
    }
}

/// Get last load result
pub fn last_load_result() -> u32 {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.last_load_result
}

/// Get random seed if present
pub fn random_seed() -> Option<[u64; 2]> {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.random_seed
}

/// Get user ID storage pointer if present
pub fn user_id_storage() -> Option<NonNull<AccountUid>> {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.user_id_storage
}

/// Returns true if chain loading is supported
pub fn has_next_load() -> bool {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.has_next_load
}

/// Get service overrides as a slice of Options (first `count` are Some)
pub fn service_overrides() -> &'static [Option<ServiceOverride>] {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    &state.service_overrides[..state.service_override_count]
}

/// Get applet type
pub fn applet_type() -> AppletType {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.applet_type
}

/// Returns true if APT workaround is active (APT is broken and should not be used)
pub fn applet_workaround() -> bool {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.applet_workaround
}

/// Set next NRO to load (chain loading)
///
/// Returns 0 on success, non-zero on error
///
/// # Safety
///
/// The caller must ensure that `path` and `argv` (if not null) point to valid,
/// null-terminated C strings that remain valid for the duration of this call.
pub unsafe fn set_next_load(path: *const c_char, argv: *const c_char) -> u32 {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };

    if !state.has_next_load {
        return 1; // Chain loading not supported
    }

    // Lock mutex to protect buffer access
    NEXT_LOAD.mutex.lock();

    // SAFETY: We hold the mutex, so we have exclusive access to the buffers
    let path_buf = unsafe { &mut *NEXT_LOAD.path.get() };
    let argv_buf = unsafe { &mut *NEXT_LOAD.argv.get() };

    // Copy path string
    if !path.is_null() {
        let mut i = 0;
        while i < path_buf.len() - 1 {
            // SAFETY: Caller guarantees path points to a valid null-terminated C string.
            // We stop at the first null byte or buffer limit, whichever comes first.
            let byte = unsafe { *path.add(i) };
            path_buf[i] = byte;
            if byte == 0 {
                break;
            }
            i += 1;
        }
        path_buf[i] = 0; // Ensure null termination
    } else {
        path_buf[0] = 0;
    }

    // Copy argv string
    if !argv.is_null() {
        let mut i = 0;
        while i < argv_buf.len() - 1 {
            // SAFETY: Caller guarantees argv points to a valid null-terminated C string.
            // We stop at the first null byte or buffer limit, whichever comes first.
            let byte = unsafe { *argv.add(i) };
            argv_buf[i] = byte;
            if byte == 0 {
                break;
            }
            i += 1;
        }
        argv_buf[i] = 0; // Ensure null termination
    } else {
        argv_buf[0] = 0;
    }

    NEXT_LOAD.mutex.unlock();

    0 // Success
}

/// Static storage for parsed environment state.
///
/// Filled exactly once by the calling entry crate via the [`init_once`]
/// closure, then immutable. The fields are public so an entry crate in a
/// different crate can populate them inside that closure; the only `&mut`
/// path to an `EnvState` is `init_once`, so writes stay confined to the
/// one-shot initialization.
pub struct EnvState {
    /// True if running as NSO (system module), false if NRO (homebrew)
    pub is_nso: bool,

    /// Heap override (address, size)
    pub heap_override: Option<(NonNull<c_void>, usize)>,

    /// Argv string pointer
    pub argv: Option<NonNull<c_char>>,

    /// Thread and process handles
    pub main_thread_handle: Option<ThreadHandle>,
    pub process_handle: Option<ProcessHandle>,

    /// Syscall availability hints (192 bits for SVCs 0x00-0xBF)
    pub syscall_hints: Option<SyscallHints>,

    /// Random seed data
    pub random_seed: Option<[u64; 2]>,

    /// Last load result
    pub last_load_result: u32,

    /// Loader info string (pointer, size)
    pub loader_info: Option<(NonNull<c_char>, u64)>,

    /// User ID storage pointer
    pub user_id_storage: Option<NonNull<AccountUid>>,

    /// Chain loading capability flag (set once during init)
    pub has_next_load: bool,

    /// Service override entries from loader
    pub service_overrides: [Option<ServiceOverride>; MAX_SERVICE_OVERRIDES],
    pub service_override_count: usize,

    /// Applet type from loader
    pub applet_type: AppletType,

    /// APT workaround flag (true if APT is broken and should not be used)
    pub applet_workaround: bool,
}

impl EnvState {
    const fn new() -> Self {
        Self {
            is_nso: false,
            heap_override: None,
            argv: None,
            main_thread_handle: None,
            process_handle: None,
            syscall_hints: None,
            random_seed: None,
            last_load_result: 0,
            loader_info: None,
            user_id_storage: None,
            has_next_load: false,
            service_overrides: [None; MAX_SERVICE_OVERRIDES],
            service_override_count: 0,
            applet_type: AppletType::Default,
            applet_workaround: false,
        }
    }
}

/// A service override entry (name + handle)
#[derive(Clone, Copy, Debug)]
pub struct ServiceOverride {
    pub name: ServiceName,
    pub handle: ServiceHandle,
}

impl ServiceOverride {
    /// Create a new service override entry.
    pub const fn new(name: ServiceName, handle: ServiceHandle) -> Self {
        Self { name, handle }
    }
}

/// Account UserId structure (matches libnx AccountUid)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AccountUid {
    pub uid: [u64; 2],
}

/// Applet type values (matches libnx AppletType enum)
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppletType {
    /// Default/unset
    Default = -1,
    /// Regular application
    Application = 0,
    /// System applet
    SystemApplet = 1,
    /// Library applet
    LibraryApplet = 2,
    /// Overlay applet
    OverlayApplet = 3,
    /// System application
    SystemApplication = 4,
}

impl AppletType {
    /// Applet flags: ApplicationOverride bit
    const FLAG_APPLICATION_OVERRIDE: u64 = 1 << 0;

    /// Create from raw loader values, applying flags
    pub const fn from_raw(value: u32, flags: u64) -> Self {
        let mut applet_type = match value {
            0 => Self::Application,
            1 => Self::SystemApplet,
            2 => Self::LibraryApplet,
            3 => Self::OverlayApplet,
            4 => Self::SystemApplication,
            _ => Self::Default,
        };

        // Apply ApplicationOverride flag if applicable
        if (flags & Self::FLAG_APPLICATION_OVERRIDE) != 0
            && matches!(applet_type, Self::SystemApplication)
        {
            applet_type = Self::Application;
        }

        applet_type
    }

    /// Get raw value for FFI
    pub const fn as_raw(self) -> u32 {
        self as i32 as u32
    }
}

/// Global environment state wrapped in UnsafeCell for interior mutability
struct EnvStateWrapper(UnsafeCell<EnvState>);

impl EnvStateWrapper {
    const fn new() -> Self {
        Self(UnsafeCell::new(EnvState::new()))
    }

    /// Get mutable access to the environment state
    ///
    /// # Safety
    ///
    /// The returned pointer grants a one-shot `&mut EnvState`. It is sound to
    /// dereference only from inside `init_once`'s `Once::call_once`, which
    /// guarantees exactly one writer with no concurrent readers. `init_once`
    /// is the sole caller; the [`EnvState`] is read-only afterwards.
    unsafe fn get(&self) -> *mut EnvState {
        // SAFETY: Caller dereferences this only within Once::call_once.
        self.0.get()
    }

    /// Get immutable access to the environment state
    ///
    /// # Safety
    ///
    /// Caller must ensure the state has been initialized via init_once()
    /// before calling this method.
    unsafe fn get_ref(&self) -> &EnvState {
        // SAFETY: Caller guarantees initialization has completed
        unsafe { &*self.0.get() }
    }
}

unsafe impl Sync for EnvStateWrapper {}

/// Chain loading state (mutable at runtime)
struct NextLoadState {
    path: UnsafeCell<[u8; 512]>,
    argv: UnsafeCell<[u8; 2048]>,
    mutex: Mutex,
}

impl NextLoadState {
    const fn new() -> Self {
        Self {
            path: UnsafeCell::new([0; 512]),
            argv: UnsafeCell::new([0; 2048]),
            mutex: Mutex::new(),
        }
    }
}

unsafe impl Sync for NextLoadState {}
