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

use core::{
    cell::UnsafeCell,
    ffi::{c_char, c_void},
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, Ordering},
};

use nx_sys_sync::{Mutex, Once};

/// Loader return function type
pub type LoaderReturnFn = Option<unsafe extern "C" fn(i32) -> !>;

/// Global environment state (immutable after initialization)
static ENV_STATE: EnvStateWrapper = EnvStateWrapper::new();

/// Initialization guard to ensure env_setup runs exactly once
static ENV_INIT: Once = Once::new();

/// Exit function pointer (mutable at runtime)
static EXIT_FUNC: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

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

static NEXT_LOAD: NextLoadState = NextLoadState::new();

/// Parse the homebrew loader environment configuration
///
/// # Safety
///
/// This function must be called exactly once during initialization.
/// The `ctx` pointer must either be null (NSO mode) or point to a valid
/// ConfigEntry array terminated by EndOfList.
pub unsafe fn setup(ctx: *const ConfigEntry, main_thread: u32, saved_lr: LoaderReturnFn) {
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
unsafe fn env_init_nso(state: &mut EnvState, main_thread: u32, saved_lr: LoaderReturnFn) {
    // Initialize exit function pointer
    let exit_ptr = match saved_lr {
        None => ptr::null_mut(),
        Some(f) => f as *mut c_void,
    };
    EXIT_FUNC.store(exit_ptr, Ordering::Relaxed);

    // NSO mode
    state.is_nso = true;
    state.main_thread_handle = main_thread;

    // In NSO mode, all syscalls are hinted as available
    state.syscall_hints[0] = u64::MAX;
    state.syscall_hints[1] = u64::MAX;
    state.syscall_hints[2] = u64::MAX;
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

    let mut entry_ptr = ctx.as_ptr();
    loop {
        // SAFETY: Caller guarantees ctx points to a valid ConfigEntry array.
        // entry_ptr starts at ctx and only advances via add(1) below.
        let entry = unsafe { &*entry_ptr };

        match EntryType::from_u32(entry.key) {
            Some(EntryType::MainThreadHandle) => {
                state.main_thread_handle = entry.value[0] as u32;
            }
            Some(EntryType::NextLoadPath) => {
                state.has_next_load = true;
                // Value[0] and Value[1] are buffer pointers set by loader
            }
            Some(EntryType::OverrideHeap) => {
                state.heap_override = NonNull::new(entry.value[0] as *mut c_void)
                    .map(|addr| (addr, entry.value[1] as usize));
            }
            Some(EntryType::Argv) => {
                state.argv = NonNull::new(entry.value[1] as *mut c_char);
            }
            Some(EntryType::SyscallAvailableHint) => {
                // SVCs 0x00-0x7F
                state.syscall_hints[0] = entry.value[0];
                state.syscall_hints[1] = entry.value[1];
            }
            Some(EntryType::SyscallAvailableHint2) => {
                // SVCs 0x80-0xBF
                state.syscall_hints[2] = entry.value[0];
            }
            Some(EntryType::ProcessHandle) => {
                state.process_handle = entry.value[0] as u32;
            }
            Some(EntryType::LastLoadResult) => {
                state.last_load_result = entry.value[0] as u32;
            }
            Some(EntryType::RandomSeed) => {
                state.random_seed = Some([entry.value[0], entry.value[1]]);
            }
            Some(EntryType::UserIdStorage) => {
                state.user_id_storage = NonNull::new(entry.value[0] as *mut AccountUid);
            }
            Some(EntryType::EndOfList) => {
                // Loader info is in the final entry's Value fields
                if entry.value[1] > 0 {
                    state.loader_info = NonNull::new(entry.value[0] as *mut c_char)
                        .map(|ptr| (ptr, entry.value[1]));
                }
                break;
            }
            _ => {
                // Ignore unknown entry types
            }
        }

        // SAFETY: The array is terminated by EndOfList, which breaks the loop.
        // We only advance the pointer if we haven't reached the end marker yet.
        entry_ptr = unsafe { entry_ptr.add(1) };
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
pub fn main_thread_handle() -> u32 {
    // SAFETY: ENV_STATE is initialized once via setup() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.main_thread_handle
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

/// Returns true if the given syscall is hinted as available
pub fn is_syscall_hinted(svc: u32) -> bool {
    if svc >= 192 {
        return false;
    }

    // SAFETY: ENV_STATE is initialized once via setup() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    let hint_index = (svc / 64) as usize;
    let bit_index = svc % 64;

    (state.syscall_hints[hint_index] & (1u64 << bit_index)) != 0
}

/// Get process handle
pub fn own_process_handle() -> u32 {
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
    main_thread_handle: u32,
    process_handle: u32,

    /// Syscall availability hints (192 bits for SVCs 0x00-0xBF)
    /// Each bit represents a syscall: bit 0 = SVC 0, bit 1 = SVC 1, etc.
    syscall_hints: [u64; 3],

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
}

impl EnvState {
    const fn new() -> Self {
        Self {
            is_nso: false,
            heap_override: None,
            argv: None,
            main_thread_handle: 0,
            process_handle: 0,
            syscall_hints: [0; 3],
            random_seed: None,
            last_load_result: 0,
            loader_info: None,
            user_id_storage: None,
            has_next_load: false,
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

/// Entry type in the homebrew environment configuration
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    /// Entry list terminator
    EndOfList = 0,
    /// Provides the handle to the main thread
    MainThreadHandle = 1,
    /// Provides a buffer containing information about the next homebrew application to load
    NextLoadPath = 2,
    /// Provides heap override information (address and size)
    OverrideHeap = 3,
    /// Provides service override information
    OverrideService = 4,
    /// Provides argv string pointer
    Argv = 5,
    /// Provides syscall availability hints for SVCs 0x00-0x7F
    SyscallAvailableHint = 6,
    /// Provides APT applet type
    AppletType = 7,
    /// Indicates that APT is broken and should not be used
    AppletWorkaround = 8,
    /// Unused/reserved entry type (formerly used by StdioSockets)
    Reserved9 = 9,
    /// Provides the process handle
    ProcessHandle = 10,
    /// Provides the last load result code
    LastLoadResult = 11,
    /// Provides random data used to seed the pseudo-random number generator
    RandomSeed = 14,
    /// Provides persistent storage for the preselected user id
    UserIdStorage = 15,
    /// Provides the currently running Horizon OS version
    HosVersion = 16,
    /// Provides syscall availability hints for SVCs 0x80-0xBF
    SyscallAvailableHint2 = 17,
}

impl EntryType {
    /// Convert from u32 to EntryType
    pub const fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::EndOfList),
            1 => Some(Self::MainThreadHandle),
            2 => Some(Self::NextLoadPath),
            3 => Some(Self::OverrideHeap),
            4 => Some(Self::OverrideService),
            5 => Some(Self::Argv),
            6 => Some(Self::SyscallAvailableHint),
            7 => Some(Self::AppletType),
            8 => Some(Self::AppletWorkaround),
            9 => Some(Self::Reserved9),
            10 => Some(Self::ProcessHandle),
            11 => Some(Self::LastLoadResult),
            14 => Some(Self::RandomSeed),
            15 => Some(Self::UserIdStorage),
            16 => Some(Self::HosVersion),
            17 => Some(Self::SyscallAvailableHint2),
            _ => None,
        }
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
