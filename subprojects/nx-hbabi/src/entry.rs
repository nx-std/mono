//! One entry of the handover, in both the shape it travels in and the shape it
//! means.

use core::{
    ffi::{
        c_char,
        c_void,
    },
    ptr::NonNull,
};

use nx_sf::ServiceName;
use nx_svc::raw::Handle as RawHandle;

/// A single fact the loader is telling the program, with the fields its key
/// gives the payload words.
///
/// This is the typed view of a [`ConfigEntry`], and the one place the key table
/// is written down. Reading is `Entry::from(config_entry)` and writing is
/// `ConfigEntry::from(entry)`. Encoding an entry and decoding it back yields
/// the entry again, for every variant including [`Unknown`](Self::Unknown),
/// which is what keeps the two sides of the handover from drifting apart.
///
/// Every pointer arrives as an address the loader chose and is `Option` because
/// a null one is how the loader says it has nothing to point at. Handles arrive
/// as the bare number: naming a handle closes nothing, and deciding who owns it
/// is the consumer's call, not the format's.
#[derive(Debug, Clone, Copy)]
pub enum Entry {
    /// Free-form text naming the loader, and the list terminator.
    ///
    /// The two jobs are one entry because the format has no separate end
    /// marker: reaching this key is what tells a program the list is over,
    /// whether or not the loader put any text in it.
    LoaderInfo {
        /// The text, which is not NUL-terminated - `len` is the length.
        text: Option<NonNull<c_char>>,
        len: u64,
    },
    /// The handle naming the process's main thread.
    MainThreadHandle(RawHandle),
    /// The handle naming the process itself.
    ProcessHandle(RawHandle),
    /// The loader's buffers for naming the program to run next.
    ///
    /// Both are buffers the loader owns and the program writes into, which is
    /// how a program asks to be replaced rather than exited. Their presence is
    /// what makes the request possible at all: a loader that offers no buffers
    /// is one that will not chain-load.
    NextLoadPath {
        path: Option<NonNull<c_char>>,
        argv: Option<NonNull<c_char>>,
    },
    /// The heap the program must use, in place of asking the kernel for one.
    OverrideHeap {
        addr: Option<NonNull<c_void>>,
        size: usize,
    },
    /// A service session the loader has already opened on the program's behalf.
    ///
    /// A program that looks this name up must be handed this session rather
    /// than one it opens itself, which is how a loader interposes on a service.
    /// The key may appear many times, once per name.
    OverrideService {
        name: ServiceName,
        handle: RawHandle,
    },
    /// The command line, as one NUL-terminated string to be split by the
    /// program.
    Argv(Option<NonNull<c_char>>),
    /// Which of supervisor calls `0x00..=0x7F` the program may make.
    ///
    /// A bit per call, lowest call in the lowest bit. A loader clears the bits
    /// for calls the kernel it is running under will reject, so a program can
    /// pick a fallback instead of taking the fault.
    SyscallHint { hint_0_3f: u64, hint_40_7f: u64 },
    /// Which of supervisor calls `0x80..=0xBF` the program may make, on the
    /// same terms as [`SyscallHint`](Self::SyscallHint).
    SyscallHint2 { hint_80_bf: u64 },
    /// Which kind of applet the process was started as, and flags refining it.
    AppletType { kind: u32, flags: u64 },
    /// The applet service is not usable in this environment and must be left
    /// alone.
    ///
    /// The entry carries no payload: its presence is the whole message.
    AppletWorkaround,
    /// Storage the loader keeps for the preselected user id, which persists
    /// across a chain load.
    ///
    /// The address holds a 16-byte account user id.
    UserIdStorage(Option<NonNull<u8>>),
    /// How the previously chain-loaded program ended.
    LastLoadResult(u32),
    /// Entropy for seeding the program's pseudo-random number generator.
    RandomSeed([u64; 2]),
    /// The Horizon OS version the process is running under.
    ///
    /// `is_atmosphere` is the loader reporting a custom firmware rather than a
    /// version digit, and travels in the second word as a magic value rather
    /// than a bit, which is why it is decoded to a `bool` here.
    HosVersion { version: u32, is_atmosphere: bool },
    /// A key this crate does not know.
    ///
    /// Kept rather than dropped because of `flags`: an unknown entry the loader
    /// marked [mandatory](EntryFlags::MANDATORY) is one the program must return
    /// to the loader over, and the payload is preserved so a consumer that does
    /// know the key can still read it.
    Unknown {
        key: Key,
        flags: EntryFlags,
        value: [u64; 2],
    },
}

impl From<ConfigEntry> for Entry {
    fn from(entry: ConfigEntry) -> Self {
        // Every payload field narrower than the 64-bit word carrying it sits in
        // the low half, so these narrowing casts drop bits the format leaves
        // zero.
        match entry.key {
            Key::LOADER_INFO => Self::LoaderInfo {
                text: NonNull::new(entry.value[0] as *mut c_char),
                len: entry.value[1],
            },
            Key::MAIN_THREAD_HANDLE => Self::MainThreadHandle(entry.value[0] as RawHandle),
            Key::PROCESS_HANDLE => Self::ProcessHandle(entry.value[0] as RawHandle),
            Key::NEXT_LOAD_PATH => Self::NextLoadPath {
                path: NonNull::new(entry.value[0] as *mut c_char),
                argv: NonNull::new(entry.value[1] as *mut c_char),
            },
            Key::OVERRIDE_HEAP => Self::OverrideHeap {
                addr: NonNull::new(entry.value[0] as *mut c_void),
                size: entry.value[1] as usize,
            },
            Key::OVERRIDE_SERVICE => Self::OverrideService {
                // SAFETY: the format packs a NUL-padded ASCII name into this
                // word, so it is already the byte pattern the checked
                // constructor would accept.
                name: ServiceName::from_u64_unchecked(entry.value[0]),
                handle: entry.value[1] as RawHandle,
            },
            // The command line travels in the second word, not the first.
            Key::ARGV => Self::Argv(NonNull::new(entry.value[1] as *mut c_char)),
            Key::SYSCALL_HINT => Self::SyscallHint {
                hint_0_3f: entry.value[0],
                hint_40_7f: entry.value[1],
            },
            Key::SYSCALL_HINT2 => Self::SyscallHint2 {
                hint_80_bf: entry.value[0],
            },
            Key::APPLET_TYPE => Self::AppletType {
                kind: entry.value[0] as u32,
                flags: entry.value[1],
            },
            Key::APPLET_WORKAROUND => Self::AppletWorkaround,
            Key::USER_ID_STORAGE => Self::UserIdStorage(NonNull::new(entry.value[0] as *mut u8)),
            Key::LAST_LOAD_RESULT => Self::LastLoadResult(entry.value[0] as u32),
            Key::RANDOM_SEED => Self::RandomSeed(entry.value),
            Key::HOS_VERSION => Self::HosVersion {
                version: entry.value[0] as u32,
                is_atmosphere: entry.value[1] == ATMOSPHERE_MAGIC,
            },
            key => Self::Unknown {
                key,
                flags: entry.flags,
                value: entry.value,
            },
        }
    }
}

impl From<Entry> for ConfigEntry {
    fn from(entry: Entry) -> Self {
        // Flags are not the entry's to decide: only `Unknown` carries any, by
        // having preserved what it was handed. A loader marking an entry
        // mandatory does so when it appends it to the list.
        match entry {
            Entry::LoaderInfo { text, len } => Self::new(Key::LOADER_INFO, [address_of(text), len]),
            Entry::MainThreadHandle(handle) => {
                Self::new(Key::MAIN_THREAD_HANDLE, [u64::from(handle), 0])
            }
            Entry::ProcessHandle(handle) => Self::new(Key::PROCESS_HANDLE, [u64::from(handle), 0]),
            Entry::NextLoadPath { path, argv } => {
                Self::new(Key::NEXT_LOAD_PATH, [address_of(path), address_of(argv)])
            }
            Entry::OverrideHeap { addr, size } => {
                Self::new(Key::OVERRIDE_HEAP, [address_of(addr), size as u64])
            }
            Entry::OverrideService { name, handle } => {
                Self::new(Key::OVERRIDE_SERVICE, [name.to_u64(), u64::from(handle)])
            }
            Entry::Argv(argv) => Self::new(Key::ARGV, [0, address_of(argv)]),
            Entry::SyscallHint {
                hint_0_3f,
                hint_40_7f,
            } => Self::new(Key::SYSCALL_HINT, [hint_0_3f, hint_40_7f]),
            Entry::SyscallHint2 { hint_80_bf } => Self::new(Key::SYSCALL_HINT2, [hint_80_bf, 0]),
            Entry::AppletType { kind, flags } => {
                Self::new(Key::APPLET_TYPE, [u64::from(kind), flags])
            }
            Entry::AppletWorkaround => Self::new(Key::APPLET_WORKAROUND, [0, 0]),
            Entry::UserIdStorage(storage) => {
                Self::new(Key::USER_ID_STORAGE, [address_of(storage), 0])
            }
            Entry::LastLoadResult(result) => {
                Self::new(Key::LAST_LOAD_RESULT, [u64::from(result), 0])
            }
            Entry::RandomSeed(seed) => Self::new(Key::RANDOM_SEED, seed),
            Entry::HosVersion {
                version,
                is_atmosphere,
            } => Self::new(
                Key::HOS_VERSION,
                [
                    u64::from(version),
                    if is_atmosphere { ATMOSPHERE_MAGIC } else { 0 },
                ],
            ),
            Entry::Unknown { key, flags, value } => Self { key, flags, value },
        }
    }
}

/// Returns the address a pointer names, with a null one for absent.
///
/// Null is the format's own way of saying nothing is there, so this is the
/// inverse of the `NonNull::new` every decode arm applies.
fn address_of<T>(ptr: Option<NonNull<T>>) -> u64 {
    ptr.map_or(0, |ptr| ptr.as_ptr() as u64)
}

/// Marks the second word of a [`Key::HOS_VERSION`] entry as reporting a custom
/// firmware: `ATMOSPHR` read as little-endian ASCII.
const ATMOSPHERE_MAGIC: u64 = 0x41544d4f53504852;

/// Which fact an entry is stating.
///
/// A newtype rather than a bare `u32` because the number is only meaningful
/// against this table, and because an entry whose key this crate does not know
/// still has to carry one - see [`Entry::Unknown`].
///
/// The gaps in the numbering are keys that were assigned and withdrawn. They
/// stay unnamed: a program reaching one treats it as unknown, which is what the
/// format asks of any key it does not recognise.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(u32);

impl Key {
    /// Text naming the loader, and the end of the list.
    pub const LOADER_INFO: Self = Self(0);
    /// The main thread's handle.
    pub const MAIN_THREAD_HANDLE: Self = Self(1);
    /// Buffers naming the program to chain-load next.
    pub const NEXT_LOAD_PATH: Self = Self(2);
    /// The heap to use in place of asking the kernel.
    pub const OVERRIDE_HEAP: Self = Self(3);
    /// A service session the loader opened on the program's behalf.
    pub const OVERRIDE_SERVICE: Self = Self(4);
    /// The command line.
    pub const ARGV: Self = Self(5);
    /// Availability of supervisor calls `0x00..=0x7F`.
    pub const SYSCALL_HINT: Self = Self(6);
    /// The applet kind the process was started as.
    pub const APPLET_TYPE: Self = Self(7);
    /// The applet service is unusable here.
    pub const APPLET_WORKAROUND: Self = Self(8);
    /// The process's own handle.
    pub const PROCESS_HANDLE: Self = Self(10);
    /// How the previously chain-loaded program ended.
    pub const LAST_LOAD_RESULT: Self = Self(11);
    /// Entropy for the program's pseudo-random number generator.
    pub const RANDOM_SEED: Self = Self(14);
    /// Storage for the preselected user id.
    pub const USER_ID_STORAGE: Self = Self(15);
    /// The Horizon OS version in force.
    pub const HOS_VERSION: Self = Self(16);
    /// Availability of supervisor calls `0x80..=0xBF`.
    pub const SYSCALL_HINT2: Self = Self(17);

    /// Names the key the given number stands for.
    ///
    /// Total on purpose: a number this crate has no constant for is a key from
    /// a newer loader, which the format says to skip rather than reject.
    pub const fn new(key: u32) -> Self {
        Self(key)
    }

    /// Returns the number this key travels as.
    pub const fn to_raw(self) -> u32 {
        self.0
    }
}

/// What the loader is asking be done with an entry beyond reading it.
///
/// Only one bit is assigned. The rest are kept rather than masked away, so an
/// entry from a newer loader round-trips unchanged and a consumer that knows a
/// bit this crate does not can still find it.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryFlags(u32);

impl EntryFlags {
    /// Nothing beyond reading it.
    pub const NONE: Self = Self(0);
    /// The program must act on this entry or return to the loader.
    ///
    /// See the crate docs for why skipping one is not a safe default.
    pub const MANDATORY: Self = Self(1 << 0);

    /// Names the flags the given word stands for.
    pub const fn new(flags: u32) -> Self {
        Self(flags)
    }

    /// Returns whether the loader requires this entry be acted on.
    pub const fn is_mandatory(self) -> bool {
        self.0 & Self::MANDATORY.0 != 0
    }

    /// Returns the word these flags travel as.
    pub const fn to_raw(self) -> u32 {
        self.0
    }
}

/// One entry as it sits in memory, which is the layout both sides agree on.
///
/// The payload words mean nothing without the key, so read one through [`Entry`]
/// rather than reaching into `value`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ConfigEntry {
    pub key: Key,
    pub flags: EntryFlags,
    pub value: [u64; 2],
}

impl ConfigEntry {
    /// Assembles an entry stating `key`, asking nothing beyond that it be read.
    ///
    /// Loaders reach for [`ConfigListBuilder`](crate::ConfigListBuilder)
    /// instead, which is what applies flags and terminates the list.
    const fn new(key: Key, value: [u64; 2]) -> Self {
        Self {
            key,
            flags: EntryFlags::NONE,
            value,
        }
    }

    /// Returns the same entry with `flags` in place of its own.
    pub const fn with_flags(mut self, flags: EntryFlags) -> Self {
        self.flags = flags;
        self
    }
}
