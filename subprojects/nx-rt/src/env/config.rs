//! Homebrew ABI configuration entry parsing.
//!
//! This module provides parsing types and iterators for the homebrew ABI environment
//! configuration passed by libnx's CRT0.

use core::{
    ffi::{c_char, c_void},
    ptr::NonNull,
};

/// Atmosphere magic value in entry.value[1]: 'ATMOSPHR' in little-endian
const ATMOSPHERE_MAGIC: u64 = 0x41544d4f53504852;

/// Maximum possible config entries (15 single-instance types + 32 service overrides + 1 margin)
const MAX_CONFIG_ENTRIES: usize = 48;

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
