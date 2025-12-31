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
    ptr,
};

use nx_sys_sync::Once;

/// Entry list terminator
pub const ENTRY_TYPE_END_OF_LIST: u32 = 0;

/// Provides the handle to the main thread
pub const ENTRY_TYPE_MAIN_THREAD_HANDLE: u32 = 1;

/// Provides a buffer containing information about the next homebrew application to load
pub const ENTRY_TYPE_NEXT_LOAD_PATH: u32 = 2;

/// Provides heap override information (address and size)
pub const ENTRY_TYPE_OVERRIDE_HEAP: u32 = 3;

/// Provides service override information
pub const ENTRY_TYPE_OVERRIDE_SERVICE: u32 = 4;

/// Provides argv string pointer
pub const ENTRY_TYPE_ARGV: u32 = 5;

/// Provides syscall availability hints for SVCs 0x00-0x7F
pub const ENTRY_TYPE_SYSCALL_AVAILABLE_HINT: u32 = 6;

/// Provides APT applet type
pub const ENTRY_TYPE_APPLET_TYPE: u32 = 7;

/// Indicates that APT is broken and should not be used
pub const ENTRY_TYPE_APPLET_WORKAROUND: u32 = 8;

/// Unused/reserved entry type (formerly used by StdioSockets)
pub const ENTRY_TYPE_RESERVED9: u32 = 9;

/// Provides the process handle
pub const ENTRY_TYPE_PROCESS_HANDLE: u32 = 10;

/// Provides the last load result code
pub const ENTRY_TYPE_LAST_LOAD_RESULT: u32 = 11;

/// Provides random data used to seed the pseudo-random number generator
pub const ENTRY_TYPE_RANDOM_SEED: u32 = 14;

/// Provides persistent storage for the preselected user id
pub const ENTRY_TYPE_USER_ID_STORAGE: u32 = 15;

/// Provides the currently running Horizon OS version
pub const ENTRY_TYPE_HOS_VERSION: u32 = 16;

/// Provides syscall availability hints for SVCs 0x80-0xBF
pub const ENTRY_TYPE_SYSCALL_AVAILABLE_HINT2: u32 = 17;

/// Loader return function type
pub type LoaderReturnFn = Option<unsafe extern "C" fn(i32) -> !>;

/// Global environment state
static ENV_STATE: EnvStateWrapper = EnvStateWrapper::new();

/// Initialization guard to ensure env_setup runs exactly once
static ENV_INIT: Once = Once::new();

/// Parse the homebrew loader environment configuration
///
/// # Safety
///
/// This function must be called exactly once during initialization.
/// The `ctx` pointer must either be null (NSO mode) or point to a valid
/// ConfigEntry array terminated by ENTRY_TYPE_END_OF_LIST.
pub unsafe fn env_setup(ctx: *const ConfigEntry, main_thread: u32, saved_lr: LoaderReturnFn) {
    // Use Once to ensure this only runs once
    ENV_INIT.call_once(|| {
        let state = unsafe { ENV_STATE.get() };

        // Check if running as NSO (ctx is null) or NRO (ctx is valid)
        if ctx.is_null() {
            // NSO mode
            state.is_nso = true;
            state.exit_func = saved_lr;
            state.main_thread_handle = main_thread;

            // In NSO mode, all syscalls are hinted as available
            state.syscall_hints[0] = u64::MAX;
            state.syscall_hints[1] = u64::MAX;
            state.syscall_hints[2] = u64::MAX;
        } else {
            // NRO mode - parse ConfigEntry array
            state.is_nso = false;
            state.exit_func = saved_lr;

            let mut entry_ptr = ctx;
            loop {
                let entry = unsafe { &*entry_ptr };

                match entry.key {
                    ENTRY_TYPE_END_OF_LIST => {
                        // Loader info is in the final entry's Value fields
                        state.loader_info = entry.value[0] as *const c_char;
                        state.loader_info_size = entry.value[1];
                        break;
                    }
                    ENTRY_TYPE_MAIN_THREAD_HANDLE => {
                        state.main_thread_handle = entry.value[0] as u32;
                    }
                    ENTRY_TYPE_NEXT_LOAD_PATH => {
                        state.has_next_load = true;
                        // Value[0] and Value[1] are buffer pointers set by loader
                    }
                    ENTRY_TYPE_OVERRIDE_HEAP => {
                        state.has_heap_override = true;
                        state.override_heap_addr = entry.value[0] as *mut c_void;
                        state.override_heap_size = entry.value[1];
                    }
                    ENTRY_TYPE_ARGV => {
                        state.override_argv = entry.value[1] as *const c_char;
                    }
                    ENTRY_TYPE_SYSCALL_AVAILABLE_HINT => {
                        // SVCs 0x00-0x7F
                        state.syscall_hints[0] = entry.value[0];
                        state.syscall_hints[1] = entry.value[1];
                    }
                    ENTRY_TYPE_SYSCALL_AVAILABLE_HINT2 => {
                        // SVCs 0x80-0xBF
                        state.syscall_hints[2] = entry.value[0];
                    }
                    ENTRY_TYPE_PROCESS_HANDLE => {
                        state.process_handle = entry.value[0] as u32;
                    }
                    ENTRY_TYPE_LAST_LOAD_RESULT => {
                        state.last_load_result = entry.value[0] as u32;
                    }
                    ENTRY_TYPE_RANDOM_SEED => {
                        state.has_random_seed = true;
                        state.random_seed[0] = entry.value[0];
                        state.random_seed[1] = entry.value[1];
                    }
                    ENTRY_TYPE_USER_ID_STORAGE => {
                        state.user_id_storage = entry.value[0] as *mut AccountUid;
                    }
                    _ => {
                        // Ignore unknown entry types
                    }
                }

                entry_ptr = unsafe { entry_ptr.add(1) };
            }
        }
    });
}

/// Get loader info string pointer
pub fn env_get_loader_info() -> *const c_char {
    let state = unsafe { ENV_STATE.get_ref() };
    state.loader_info
}

/// Get loader info size
pub fn env_get_loader_info_size() -> u64 {
    let state = unsafe { ENV_STATE.get_ref() };
    state.loader_info_size
}

/// Get main thread handle
pub fn env_get_main_thread_handle() -> u32 {
    let state = unsafe { ENV_STATE.get_ref() };
    state.main_thread_handle
}

/// Returns true if running as NSO, false if NRO
pub fn env_is_nso() -> bool {
    let state = unsafe { ENV_STATE.get_ref() };
    state.is_nso
}

/// Returns true if heap override is present
pub fn env_has_heap_override() -> bool {
    let state = unsafe { ENV_STATE.get_ref() };
    state.has_heap_override
}

/// Get heap override address
pub fn env_get_heap_override_addr() -> *mut c_void {
    let state = unsafe { ENV_STATE.get_ref() };
    state.override_heap_addr
}

/// Get heap override size
pub fn env_get_heap_override_size() -> u64 {
    let state = unsafe { ENV_STATE.get_ref() };
    state.override_heap_size
}

/// Returns true if argv is present
pub fn env_has_argv() -> bool {
    let state = unsafe { ENV_STATE.get_ref() };
    !state.override_argv.is_null()
}

/// Get argv string pointer
pub fn env_get_argv() -> *const c_char {
    let state = unsafe { ENV_STATE.get_ref() };
    state.override_argv
}

/// Returns true if the given syscall is hinted as available
pub fn env_is_syscall_hinted(svc: u32) -> bool {
    if svc >= 192 {
        return false;
    }

    let state = unsafe { ENV_STATE.get_ref() };
    let hint_index = (svc / 64) as usize;
    let bit_index = svc % 64;

    (state.syscall_hints[hint_index] & (1u64 << bit_index)) != 0
}

/// Get process handle
pub fn env_get_own_process_handle() -> u32 {
    let state = unsafe { ENV_STATE.get_ref() };
    state.process_handle
}

/// Get exit function pointer
pub fn env_get_exit_func_ptr() -> LoaderReturnFn {
    let state = unsafe { ENV_STATE.get_ref() };
    state.exit_func
}

/// Set exit function pointer
pub fn env_set_exit_func_ptr(func: LoaderReturnFn) {
    let state = unsafe { ENV_STATE.get() };
    state.exit_func = func;
}

/// Returns true if chain loading is supported
pub fn env_has_next_load() -> bool {
    let state = unsafe { ENV_STATE.get_ref() };
    state.has_next_load
}

/// Get last load result
pub fn env_get_last_load_result() -> u32 {
    let state = unsafe { ENV_STATE.get_ref() };
    state.last_load_result
}

/// Returns true if random seed is present
pub fn env_has_random_seed() -> bool {
    let state = unsafe { ENV_STATE.get_ref() };
    state.has_random_seed
}

/// Get random seed (copies to output buffer)
pub fn env_get_random_seed(out: &mut [u64; 2]) {
    let state = unsafe { ENV_STATE.get_ref() };
    *out = state.random_seed;
}

/// Get user ID storage pointer
pub fn env_get_user_id_storage() -> *mut AccountUid {
    let state = unsafe { ENV_STATE.get_ref() };
    state.user_id_storage
}

/// Set next NRO to load (chain loading)
///
/// Returns 0 on success, non-zero on error
pub fn env_set_next_load(path: *const c_char, argv: *const c_char) -> u32 {
    let state = unsafe { ENV_STATE.get() };

    if !state.has_next_load {
        return 1; // Chain loading not supported
    }

    // Copy path string
    if !path.is_null() {
        let mut i = 0;
        while i < state.next_load_path.len() - 1 {
            let byte = unsafe { *path.add(i) } as u8;
            state.next_load_path[i] = byte;
            if byte == 0 {
                break;
            }
            i += 1;
        }
        state.next_load_path[i] = 0; // Ensure null termination
    } else {
        state.next_load_path[0] = 0;
    }

    // Copy argv string
    if !argv.is_null() {
        let mut i = 0;
        while i < state.next_load_argv.len() - 1 {
            let byte = unsafe { *argv.add(i) } as u8;
            state.next_load_argv[i] = byte;
            if byte == 0 {
                break;
            }
            i += 1;
        }
        state.next_load_argv[i] = 0; // Ensure null termination
    } else {
        state.next_load_argv[0] = 0;
    }

    0 // Success
}

/// Static storage for parsed environment state
struct EnvState {
    /// True if running as NSO (system module), false if NRO (homebrew)
    is_nso: bool,

    /// Heap override
    has_heap_override: bool,
    override_heap_addr: *mut c_void,
    override_heap_size: u64,

    /// Argv string pointer
    override_argv: *const c_char,

    /// Thread and process handles
    main_thread_handle: u32,
    process_handle: u32,

    /// Syscall availability hints (192 bits for SVCs 0x00-0xBF)
    /// Each bit represents a syscall: bit 0 = SVC 0, bit 1 = SVC 1, etc.
    syscall_hints: [u64; 3],

    /// Random seed data
    random_seed: [u64; 2],
    has_random_seed: bool,

    /// Exit function and last load result
    exit_func: LoaderReturnFn,
    last_load_result: u32,

    /// Loader info string (stored at end of ConfigEntry list)
    loader_info: *const c_char,
    loader_info_size: u64,

    /// User ID storage pointer
    user_id_storage: *mut AccountUid,

    /// Chain loading buffers
    next_load_path: [u8; 512],
    next_load_argv: [u8; 2048],
    has_next_load: bool,
}

impl EnvState {
    const fn new() -> Self {
        Self {
            is_nso: false,
            has_heap_override: false,
            override_heap_addr: ptr::null_mut(),
            override_heap_size: 0,
            override_argv: ptr::null(),
            main_thread_handle: 0,
            process_handle: 0,
            syscall_hints: [0; 3],
            random_seed: [0; 2],
            has_random_seed: false,
            exit_func: None,
            last_load_result: 0,
            loader_info: ptr::null(),
            loader_info_size: 0,
            user_id_storage: ptr::null_mut(),
            next_load_path: [0; 512],
            next_load_argv: [0; 2048],
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

    unsafe fn get(&self) -> &mut EnvState {
        unsafe { &mut *self.0.get() }
    }

    unsafe fn get_ref(&self) -> &EnvState {
        unsafe { &*self.0.get() }
    }
}

unsafe impl Sync for EnvStateWrapper {}

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
