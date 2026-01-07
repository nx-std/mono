//! # Runtime Environment Parser
//!
//! This crate parses the homebrew ABI environment passed by libnx's CRT0
//! and provides functions to query runtime environment information.
//!
//! It uses static memory only (no allocator dependency), making it suitable
//! for early runtime initialization before the heap is available.

#![no_std]

extern crate nx_panic_handler as _;

#[cfg(feature = "ffi")]
mod ffi;

pub mod hos_version;
pub mod main_thread;
mod syscall_hint;

use core::{
    cell::UnsafeCell,
    ffi::{c_char, c_void},
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, Ordering},
};

use nx_svc::{
    ipc::Handle as ServiceHandle, process::Handle as ProcessHandle, thread::Handle as ThreadHandle,
};
use nx_sys_sync::{Mutex, Once};
pub use syscall_hint::SyscallHints;

/// Loader return function type
pub type LoaderReturnFn = Option<unsafe extern "C" fn(i32) -> !>;

/// Atmosphere magic value in entry.value[1]: 'ATMOSPHR' in little-endian
const ATMOSPHERE_MAGIC: u64 = 0x41544d4f53504852;

/// Atmosphere flag bit position (used to set bit 31 when Atmosphere is detected)
const HOS_VERSION_ATMOSPHERE_BIT: u32 = 1 << 31;

/// Maximum number of service overrides (matches libnx MAX_OVERRIDES)
const MAX_SERVICE_OVERRIDES: usize = 32;

/// Maximum possible config entries (15 single-instance types + 32 service overrides + 1 margin)
const MAX_CONFIG_ENTRIES: usize = 48;

/// Global environment state (immutable after initialization)
static ENV_STATE: EnvStateWrapper = EnvStateWrapper::new();

/// Initialization guard to ensure env_setup runs exactly once
static ENV_INIT: Once = Once::new();

/// Exit function pointer (mutable at runtime)
static EXIT_FUNC: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

static NEXT_LOAD: NextLoadState = NextLoadState::new();

/// Parse the homebrew loader environment configuration
///
/// # Safety
///
/// This function must be called exactly once during initialization.
/// The `ctx` pointer must either be null (NSO mode) or point to a valid
/// ConfigEntry array terminated by EndOfList.
pub unsafe fn setup(ctx: *const ConfigEntry, main_thread: ThreadHandle, saved_lr: LoaderReturnFn) {
    // Use Once to ensure this only runs once
    ENV_INIT.call_once(|| {
        // SAFETY: We're inside Once::call_once, which guarantees exclusive access
        let state = unsafe { ENV_STATE.get() };

        // Select initialization based on whether ctx is null
        match NonNull::new(ctx as *mut ConfigEntry) {
            None => unsafe { env_init_nso(state, main_thread, saved_lr) },
            Some(ctx_ptr) => unsafe { env_init_nro(state, ctx_ptr, saved_lr) },
        }
    });
}

/// NSO initialization
///
/// # Safety
///
/// Must only be called from within `Once::call_once`.
unsafe fn env_init_nso(state: &mut EnvState, main_thread: ThreadHandle, saved_lr: LoaderReturnFn) {
    // Initialize exit function pointer
    let exit_ptr = match saved_lr {
        None => ptr::null_mut(),
        Some(f) => f as *mut c_void,
    };
    EXIT_FUNC.store(exit_ptr, Ordering::Relaxed);

    // NSO mode
    state.is_nso = true;
    state.main_thread_handle = Some(main_thread);

    // In NSO mode, all syscalls are hinted as available
    state.syscall_hints = Some(SyscallHints::all_available());
}

/// NRO initialization
///
/// # Safety
///
/// Must only be called from within `Once::call_once`.
/// The `ctx` pointer must point to a valid ConfigEntry array terminated by EndOfList.
unsafe fn env_init_nro(state: &mut EnvState, ctx: NonNull<ConfigEntry>, saved_lr: LoaderReturnFn) {
    // Initialize exit function pointer
    let exit_ptr = match saved_lr {
        None => ptr::null_mut(),
        Some(f) => f as *mut c_void,
    };
    EXIT_FUNC.store(exit_ptr, Ordering::Relaxed);

    // NRO mode
    state.is_nso = false;

    // SAFETY: Caller guarantees ctx points to valid ConfigEntry array terminated by EndOfList
    let entries = unsafe { ConfigEntries::from_ptr(ctx) };

    for entry in entries {
        match entry {
            Entry::HosVersion {
                version,
                is_atmosphere,
            } => {
                let mut v = version;
                if is_atmosphere {
                    v |= HOS_VERSION_ATMOSPHERE_BIT;
                }
                hos_version::set(v);
            }
            Entry::MainThreadHandle(raw) => {
                // SAFETY: The handle is provided by the loader and guaranteed valid
                state.main_thread_handle = Some(unsafe { ThreadHandle::from_raw(raw) });
            }
            Entry::ProcessHandle(raw) => {
                // SAFETY: The handle is provided by the loader and guaranteed valid
                state.process_handle = Some(unsafe { ProcessHandle::from_raw(raw) });
            }
            Entry::OverrideHeap { addr, size } => {
                state.heap_override = addr.map(|a| (a, size));
            }
            Entry::Argv(ptr) => {
                state.argv = ptr;
            }
            Entry::RandomSeed(seed) => {
                state.random_seed = Some(seed);
            }
            Entry::SyscallHint {
                hint_0_3f,
                hint_40_7f,
            } => {
                state
                    .syscall_hints
                    .get_or_insert_with(SyscallHints::new)
                    .set_hint_0_7f(hint_0_3f, hint_40_7f);
            }
            Entry::SyscallHint2 { hint_80_bf } => {
                state
                    .syscall_hints
                    .get_or_insert_with(SyscallHints::new)
                    .set_hint_80_bf(hint_80_bf);
            }
            Entry::UserIdStorage(ptr) => {
                state.user_id_storage = ptr;
            }
            Entry::LastLoadResult(result) => {
                state.last_load_result = result;
            }
            Entry::NextLoadPath => {
                state.has_next_load = true;
            }
            Entry::OverrideService { name, handle } => {
                if state.service_override_count < MAX_SERVICE_OVERRIDES {
                    // SAFETY: The handle is provided by the loader and guaranteed valid
                    let service_handle = unsafe { ServiceHandle::from_raw(handle) };
                    state.service_overrides[state.service_override_count] =
                        Some(ServiceOverride::new(name, service_handle));
                    state.service_override_count += 1;
                }
            }
            Entry::AppletType { kind, flags } => {
                state.applet_type = AppletType::from_raw(kind, flags);
            }
            Entry::AppletWorkaround => {
                state.applet_workaround = true;
            }
            Entry::LoaderInfo { ptr, len } => {
                if len > 0 {
                    state.loader_info = ptr.map(|p| (p, len));
                }
            }
            Entry::Unknown { .. } => {
                // Ignore unknown entry types
            }
        }
    }
}

/// Get loader info string pointer and size
pub fn loader_info() -> Option<(NonNull<c_char>, u64)> {
    // SAFETY: ENV_STATE is initialized once via setup() before any other function is called.
    // After initialization, the state is read-only.
    let state = unsafe { ENV_STATE.get_ref() };
    state.loader_info
}

/// Get main thread handle
///
/// # Panics
///
/// Panics if called before the environment is initialized.
pub fn main_thread_handle() -> ThreadHandle {
    // SAFETY: ENV_STATE is initialized once via setup() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state
        .main_thread_handle
        .expect("main thread handle not set")
}

/// Returns true if running as NSO, false if NRO
pub fn is_nso() -> bool {
    // SAFETY: ENV_STATE is initialized once via setup() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.is_nso
}

/// Get heap override address and size if present
///
/// Returns `Some((addr, size))` if the homebrew loader provided a heap override,
/// or `None` if running without a heap override.
pub fn heap_override() -> Option<(NonNull<c_void>, usize)> {
    // SAFETY: ENV_STATE is initialized once via setup() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.heap_override
}

/// Get argv string pointer if present
pub fn argv() -> Option<*const c_char> {
    // SAFETY: ENV_STATE is initialized once via setup() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.argv.map(|ptr| ptr.as_ptr() as *const c_char)
}

/// Get syscall availability hints
///
/// # Panics
///
/// Panics if called before the environment is initialized.
pub fn syscall_hints() -> SyscallHints {
    // SAFETY: ENV_STATE is initialized once via setup() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.syscall_hints.expect("syscall hints not set")
}

/// Get process handle if present
pub fn own_process_handle() -> Option<ProcessHandle> {
    // SAFETY: ENV_STATE is initialized once via setup() and is read-only after that.
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
        Some(unsafe { core::mem::transmute(ptr) })
    }
}

/// Get last load result
pub fn last_load_result() -> u32 {
    // SAFETY: ENV_STATE is initialized once via setup() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.last_load_result
}

/// Get random seed if present
pub fn random_seed() -> Option<[u64; 2]> {
    // SAFETY: ENV_STATE is initialized once via setup() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.random_seed
}

/// Get user ID storage pointer if present
pub fn user_id_storage() -> Option<NonNull<AccountUid>> {
    // SAFETY: ENV_STATE is initialized once via setup() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.user_id_storage
}

/// Returns true if chain loading is supported
pub fn has_next_load() -> bool {
    // SAFETY: ENV_STATE is initialized once via setup() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.has_next_load
}

/// Get service overrides as a slice of Options (first `count` are Some)
pub fn service_overrides() -> &'static [Option<ServiceOverride>] {
    // SAFETY: ENV_STATE is initialized once via setup() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    &state.service_overrides[..state.service_override_count]
}

/// Get applet type
pub fn applet_type() -> AppletType {
    // SAFETY: ENV_STATE is initialized once via setup() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.applet_type
}

/// Returns true if APT workaround is active (APT is broken and should not be used)
pub fn applet_workaround() -> bool {
    // SAFETY: ENV_STATE is initialized once via setup() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.applet_workaround
}

/// Set next NRO to load (chain loading)
///
/// Returns 0 on success, non-zero on error
pub fn set_next_load(path: *const c_char, argv: *const c_char) -> u32 {
    // SAFETY: ENV_STATE is initialized once via setup() and is read-only after that.
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
            let byte = unsafe { *path.add(i) } as u8;
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
            let byte = unsafe { *argv.add(i) } as u8;
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

/// Static storage for parsed environment state (immutable after initialization)
struct EnvState {
    /// True if running as NSO (system module), false if NRO (homebrew)
    is_nso: bool,

    /// Heap override (address, size)
    heap_override: Option<(NonNull<c_void>, usize)>,

    /// Argv string pointer
    argv: Option<NonNull<c_char>>,

    /// Thread and process handles
    main_thread_handle: Option<ThreadHandle>,
    process_handle: Option<ProcessHandle>,

    /// Syscall availability hints (192 bits for SVCs 0x00-0xBF)
    syscall_hints: Option<SyscallHints>,

    /// Random seed data
    random_seed: Option<[u64; 2]>,

    /// Last load result
    last_load_result: u32,

    /// Loader info string (pointer, size)
    loader_info: Option<(NonNull<c_char>, u64)>,

    /// User ID storage pointer
    user_id_storage: Option<NonNull<AccountUid>>,

    /// Chain loading capability flag (set once during init)
    has_next_load: bool,

    /// Service override entries from loader
    service_overrides: [Option<ServiceOverride>; MAX_SERVICE_OVERRIDES],
    service_override_count: usize,

    /// Applet type from loader
    applet_type: AppletType,

    /// APT workaround flag (true if APT is broken and should not be used)
    applet_workaround: bool,
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
    /// Caller must ensure exclusive access. This is safe when:
    /// - Called from within Once::call_once during initialization, or
    /// - Called to mutate fields that are safe to modify post-initialization
    ///   (like exit_func and next_load buffers)
    unsafe fn get(&self) -> &mut EnvState {
        // SAFETY: Caller guarantees exclusive access or safe mutation
        unsafe { &mut *self.0.get() }
    }

    /// Get immutable access to the environment state
    ///
    /// # Safety
    ///
    /// Caller must ensure the state has been initialized via setup()
    /// before calling this method.
    unsafe fn get_ref(&self) -> &EnvState {
        // SAFETY: Caller guarantees initialization has completed
        unsafe { &*self.0.get() }
    }
}

unsafe impl Sync for EnvStateWrapper {}

/// Typed view over a [`ConfigEntry`].
///
/// This enum provides type-safe access to the values in a config entry,
/// parsing the raw `[u64; 2]` values into their appropriate types.
/// Zero-copy: only extracts scalar values, no allocation.
#[derive(Debug, Clone, Copy)]
pub enum Entry {
    /// Loader info (appears in the EndOfList entry).
    LoaderInfo {
        ptr: Option<NonNull<c_char>>,
        len: u64,
    },
    /// Main thread handle (raw u32 value)
    MainThreadHandle(u32),
    /// Process handle (raw u32 value)
    ProcessHandle(u32),
    /// Chain loading path buffer available
    NextLoadPath,
    /// Heap override (address and size)
    OverrideHeap {
        addr: Option<NonNull<c_void>>,
        size: usize,
    },
    /// Service override (name and handle)
    OverrideService { name: ServiceName, handle: u32 },
    /// Argv string pointer
    Argv(Option<NonNull<c_char>>),
    /// Syscall availability hints for SVCs 0x00-0x7F
    SyscallHint { hint_0_3f: u64, hint_40_7f: u64 },
    /// Syscall availability hints for SVCs 0x80-0xBF
    SyscallHint2 { hint_80_bf: u64 },
    /// Applet type (raw kind value and flags)
    AppletType { kind: u32, flags: u64 },
    /// APT workaround flag. If present, APT is broken and should not be used.
    AppletWorkaround,
    /// User ID storage pointer
    UserIdStorage(Option<NonNull<AccountUid>>),
    /// Last load result code
    LastLoadResult(u32),
    /// Random seed data
    RandomSeed([u64; 2]),
    /// HOS version (raw version and atmosphere flag)
    HosVersion { version: u32, is_atmosphere: bool },
    /// Unknown or reserved entry type
    Unknown {
        key: u32,
        flags: u32,
        value: [u64; 2],
    },
}

impl Entry {
    /// Loader info. This is always the last entry and marks the end of the list.
    pub const KEY_LOADER_INFO: u32 = 0;
    /// Main thread handle.
    pub const KEY_MAIN_THREAD_HANDLE: u32 = 1;
    /// Next NRO path for chain loading.
    pub const KEY_NEXT_LOAD_PATH: u32 = 2;
    /// Heap override (addr, size).
    pub const KEY_OVERRIDE_HEAP: u32 = 3;
    /// Service override (name, handle).
    pub const KEY_OVERRIDE_SERVICE: u32 = 4;
    /// Argv string pointer.
    pub const KEY_ARGV: u32 = 5;
    /// Syscall availability hints for SVCs 0x00-0x7F.
    pub const KEY_SYSCALL_AVAILABLE_HINT: u32 = 6;
    /// Applet type and flags.
    pub const KEY_APPLET_TYPE: u32 = 7;
    /// APT workaround. If present, APT is broken and should not be used.
    pub const KEY_APPLET_WORKAROUND: u32 = 8;
    /// Own process handle.
    pub const KEY_PROCESS_HANDLE: u32 = 10;
    /// Previous load result code.
    pub const KEY_LAST_LOAD_RESULT: u32 = 11;
    /// PRNG seed data.
    pub const KEY_RANDOM_SEED: u32 = 14;
    /// Preselected user ID storage.
    pub const KEY_USER_ID_STORAGE: u32 = 15;
    /// Horizon OS version.
    pub const KEY_HOS_VERSION: u32 = 16;
    /// Syscall availability hints for SVCs 0x80-0xBF.
    pub const KEY_SYSCALL_AVAILABLE_HINT2: u32 = 17;

    /// Parse a [`ConfigEntry`] into a typed [`Entry`].
    pub fn from_config(entry: &ConfigEntry) -> Self {
        match entry.key {
            Self::KEY_LOADER_INFO => Entry::LoaderInfo {
                ptr: NonNull::new(entry.value[0] as *mut c_char),
                len: entry.value[1],
            },
            Self::KEY_MAIN_THREAD_HANDLE => Entry::MainThreadHandle(entry.value[0] as u32),
            Self::KEY_PROCESS_HANDLE => Entry::ProcessHandle(entry.value[0] as u32),
            Self::KEY_NEXT_LOAD_PATH => Entry::NextLoadPath,
            Self::KEY_OVERRIDE_HEAP => Entry::OverrideHeap {
                addr: NonNull::new(entry.value[0] as *mut c_void),
                size: entry.value[1] as usize,
            },
            Self::KEY_OVERRIDE_SERVICE => Entry::OverrideService {
                name: ServiceName::from_u64(entry.value[0]),
                handle: entry.value[1] as u32,
            },
            Self::KEY_ARGV => Entry::Argv(NonNull::new(entry.value[1] as *mut c_char)),
            Self::KEY_SYSCALL_AVAILABLE_HINT => Entry::SyscallHint {
                hint_0_3f: entry.value[0],
                hint_40_7f: entry.value[1],
            },
            Self::KEY_SYSCALL_AVAILABLE_HINT2 => Entry::SyscallHint2 {
                hint_80_bf: entry.value[0],
            },
            Self::KEY_APPLET_TYPE => Entry::AppletType {
                kind: entry.value[0] as u32,
                flags: entry.value[1],
            },
            Self::KEY_APPLET_WORKAROUND => Entry::AppletWorkaround,
            Self::KEY_USER_ID_STORAGE => {
                Entry::UserIdStorage(NonNull::new(entry.value[0] as *mut AccountUid))
            }
            Self::KEY_LAST_LOAD_RESULT => Entry::LastLoadResult(entry.value[0] as u32),
            Self::KEY_RANDOM_SEED => Entry::RandomSeed([entry.value[0], entry.value[1]]),
            Self::KEY_HOS_VERSION => Entry::HosVersion {
                version: entry.value[0] as u32,
                is_atmosphere: entry.value[1] == ATMOSPHERE_MAGIC,
            },
            _ => Entry::Unknown {
                key: entry.key,
                flags: entry.flags,
                value: entry.value,
            },
        }
    }
}

/// Iterator over ConfigEntry array with compile-time bound.
///
/// Stops at `EndOfList` or after `MAX_CONFIG_ENTRIES`, whichever comes first.
/// Yields parsed [`Entry`] values directly.
pub struct ConfigEntries<'a> {
    entries: &'a [ConfigEntry],
    index: usize,
    done: bool,
}

impl<'a> ConfigEntries<'a> {
    /// Create bounded iterator from raw pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a valid ConfigEntry array terminated by `EndOfList`,
    /// with at least `MAX_CONFIG_ENTRIES` readable elements OR an `EndOfList` before that.
    pub unsafe fn from_ptr(ptr: NonNull<ConfigEntry>) -> Self {
        // SAFETY: Create slice with max bound. Safe because:
        // 1. Loader guarantees EndOfList terminator
        // 2. We stop iteration at EndOfList anyway
        // 3. MAX_CONFIG_ENTRIES is the theoretical maximum
        let entries = unsafe { core::slice::from_raw_parts(ptr.as_ptr(), MAX_CONFIG_ENTRIES) };
        Self {
            entries,
            index: 0,
            done: false,
        }
    }
}

impl Iterator for ConfigEntries<'_> {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.index >= self.entries.len() {
            return None;
        }

        let entry = Entry::from_config(&self.entries[self.index]);
        self.index += 1;

        if matches!(entry, Entry::LoaderInfo { .. }) {
            self.done = true;
        }

        Some(entry)
    }
}

/// Account UserId structure (matches libnx AccountUid)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AccountUid {
    pub uid: [u64; 2],
}

/// Structure representing an entry in the homebrew environment configuration
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ConfigEntry {
    pub key: u32,
    pub flags: u32,
    pub value: [u64; 2],
}

/// Service name (8-byte null-padded string, matches libnx SmServiceName)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceName(u64);

impl ServiceName {
    /// Create from raw u64 (matches libnx's smServiceNameFromU64)
    pub const fn from_u64(val: u64) -> Self {
        Self(val)
    }

    /// Get raw u64 value
    pub const fn to_raw(self) -> u64 {
        self.0
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
        if (flags & Self::FLAG_APPLICATION_OVERRIDE) != 0 {
            if matches!(applet_type, Self::SystemApplication) {
                applet_type = Self::Application;
            }
        }

        applet_type
    }

    /// Get raw value for FFI
    pub const fn as_raw(self) -> u32 {
        self as i32 as u32
    }
}

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
