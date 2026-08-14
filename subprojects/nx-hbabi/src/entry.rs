//! One entry of the handover, in both the shape it travels in and the shape it
//! means.

use core::{
    ffi::{
        CStr,
        c_char,
    },
    ptr::NonNull,
};

use nx_sf::ServiceName;
use nx_svc::{
    ipc::Handle as SessionHandle,
    process::Handle as ProcessHandle,
    thread::Handle as ThreadHandle,
};

/// A single fact the loader is telling the program, with the fields its key
/// gives the payload words.
///
/// This is the typed view of a [`ConfigEntry`], and the one place the key table
/// is written down. Writing is `ConfigEntry::from(entry)` and reading is
/// [`decode`](Self::decode). Encoding an entry and decoding it back yields the
/// entry again, for every variant including [`Unknown`](Self::Unknown), which
/// is what keeps the two sides of the handover from drifting apart.
///
/// # What the key already settles
///
/// A payload word is an untyped 64 bits on the wire, but the key it travels
/// under says what it is, and that is exactly what this enum records. A handle
/// under [`Key::MAIN_THREAD_HANDLE`] is a thread handle, so the variant holds
/// one; the bytes a loader names itself with have an address and a length, so
/// the variant holds a slice. Nothing here hands back a number for a consumer
/// to re-derive a type from that the key had already fixed.
///
/// The handles are the naming form, which is `Copy` and closes nothing: a
/// handover tells a program which handles it has, and none of them is this
/// crate's to close.
///
/// # Why the lifetime, and why decoding is unsafe
///
/// Several entries name memory the loader owns, and this enum borrows it rather
/// than repeating an address and a length. That makes the loader's side of the
/// handover ordinary safe Rust: a loader has the slices already and hands them
/// over as they are.
///
/// The program's side cannot be safe in the same way. It receives addresses,
/// and nothing it can check says how long they are good for, so producing a
/// borrow from one is a promise only the loader's behaviour backs. That promise
/// is made once, at [`decode`](Self::decode), rather than by every consumer that
/// dereferences a pointer this enum handed it.
#[derive(Debug, Clone, Copy)]
pub enum Entry<'a> {
    /// Free-form text naming the loader, and the list terminator.
    ///
    /// The two jobs are one entry because the format has no separate end
    /// marker: reaching this key is what tells a program the list is over,
    /// whether or not the loader put any text in it.
    ///
    /// The text is not NUL-terminated, which is why it is bytes and not a
    /// [`CStr`]: the length travels in the entry.
    LoaderInfo(Option<&'a [u8]>),
    /// The handle naming the process's main thread.
    MainThreadHandle(ThreadHandle),
    /// The handle naming the process itself.
    ProcessHandle(ProcessHandle),
    /// The loader's buffers for naming the program to run next.
    ///
    /// Both are buffers the loader owns and the program *writes into*, which is
    /// how a program asks to be replaced rather than exited. Their presence is
    /// what makes the request possible at all: a loader that offers no buffers
    /// is one that will not chain-load.
    ///
    /// These stay bare addresses where the other entries carry slices, because
    /// the format gives them no length. How much a program may write is fixed
    /// by convention between the two sides, not stated in the handover, so a
    /// slice here would be a length this crate invented.
    NextLoadPath {
        path: NonNull<c_char>,
        argv: NonNull<c_char>,
    },
    /// The heap the program must use, in place of asking the kernel for one.
    ///
    /// A pointer to a region of known length rather than a slice: the memory is
    /// uninitialised and the program is about to hand it to an allocator, so
    /// nothing may read it as bytes or assume it is unaliased.
    OverrideHeap(NonNull<[u8]>),
    /// A service session the loader has already opened on the program's behalf.
    ///
    /// A program that looks this name up must be handed this session rather
    /// than one it opens itself, which is how a loader interposes on a service.
    /// The key may appear many times, once per name.
    OverrideService {
        name: ServiceName,
        handle: SessionHandle,
    },
    /// The command line, as one NUL-terminated string to be split by the
    /// program.
    Argv(&'a CStr),
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
    /// A pointer rather than a reference, because both sides use it: the
    /// program writes the id it settled on so the next one starts with it, and
    /// the loader keeps the storage between the two. The length is the id's,
    /// which the format fixes.
    UserIdStorage(NonNull<[u8; USER_ID_LEN]>),
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
    /// A key this crate knows, carrying a payload it cannot use.
    ///
    /// In practice this is one thing: an entry that must name memory and gives
    /// a null address instead. A loader with nothing to say omits the entry, so
    /// reaching here means the loader said something malformed rather than
    /// saying nothing.
    ///
    /// It is separate from [`Unknown`](Self::Unknown) because the two call for
    /// different answers. An unknown key may be from a newer loader and is
    /// skippable unless mandatory; a known key with an unusable payload is a
    /// loader that is not working, and a consumer that treats it as absent runs
    /// on a default the loader was trying to replace.
    ///
    /// The flags come along for the same reason they do on `Unknown`: a
    /// malformed entry the loader marked mandatory is still mandatory.
    Malformed {
        key: Key,
        flags: EntryFlags,
        value: [u64; 2],
    },
}

impl<'a> Entry<'a> {
    /// Reads an entry, borrowing whatever memory it names for `'a`.
    ///
    /// # Safety
    ///
    /// Every address `entry` carries must name memory this process may read for
    /// the whole of `'a`, with the length the key gives it: the loader-info
    /// bytes for the length in the entry, the command line up to its
    /// terminating NUL. A loader keeps these alive for as long as the program
    /// it started runs, which is what makes a `'static` choice of `'a` the
    /// usual one, but nothing checkable here says so.
    ///
    /// The handles are not part of that promise. Naming one closes nothing and
    /// a handle the kernel never issued is answered with `InvalidHandle` by the
    /// call that reaches it, so a wrong one is an error rather than undefined
    /// behaviour.
    pub unsafe fn decode(entry: ConfigEntry) -> Self {
        // Every payload field narrower than the 64-bit word carrying it sits in
        // the low half, so these narrowing casts drop bits the format leaves
        // zero.
        match entry.key {
            Key::LOADER_INFO => {
                let text = NonNull::new(entry.value[0] as *mut u8).map(|text| {
                    // SAFETY: the caller vouched for the entry's address naming
                    // readable memory for the length beside it, which is what
                    // this slice spans.
                    unsafe { core::slice::from_raw_parts(text.as_ptr(), entry.value[1] as usize) }
                });
                Self::LoaderInfo(text)
            }
            // SAFETY: the handle is one the loader is passing on rather than
            // one this crate is adopting, so nothing here closes it, and a
            // number the kernel did not issue is refused by the call that uses
            // it rather than faulting.
            Key::MAIN_THREAD_HANDLE => {
                Self::MainThreadHandle(ThreadHandle::from_raw_unchecked(entry.value[0] as u32))
            }
            // SAFETY: the handle is one the loader is passing on rather than
            // one this crate is adopting, so nothing here closes it, and a
            // number the kernel did not issue is refused by the call that uses
            // it rather than faulting.
            Key::PROCESS_HANDLE => {
                Self::ProcessHandle(ProcessHandle::from_raw_unchecked(entry.value[0] as u32))
            }
            Key::NEXT_LOAD_PATH => {
                // Both buffers or neither: naming a program to run without a
                // command line to run it with, or the reverse, is not a request
                // any loader can act on.
                match (
                    NonNull::new(entry.value[0] as *mut c_char),
                    NonNull::new(entry.value[1] as *mut c_char),
                ) {
                    (Some(path), Some(argv)) => Self::NextLoadPath { path, argv },
                    _ => Self::malformed(entry),
                }
            }
            Key::OVERRIDE_HEAP => match NonNull::new(entry.value[0] as *mut u8) {
                Some(addr) => {
                    Self::OverrideHeap(NonNull::slice_from_raw_parts(addr, entry.value[1] as usize))
                }
                None => Self::malformed(entry),
            },
            Key::OVERRIDE_SERVICE => Self::OverrideService {
                // SAFETY: the format packs a NUL-padded ASCII name into this
                // word, so it is already the byte pattern the checked
                // constructor would accept.
                name: ServiceName::from_u64_unchecked(entry.value[0]),
                // SAFETY: the session is one the loader opened and is passing
                // on rather than one this crate is adopting, so nothing here
                // closes it, and a number the kernel did not issue is refused
                // by the call that uses it rather than faulting.
                handle: SessionHandle::from_raw_unchecked(entry.value[1] as u32),
            },
            // The command line travels in the second word, not the first.
            Key::ARGV => match NonNull::new(entry.value[1] as *mut c_char) {
                // SAFETY: the caller vouched for the entry's address naming
                // memory readable up to a NUL, which is the run this scans.
                Some(argv) => Self::Argv(unsafe { CStr::from_ptr(argv.as_ptr()) }),
                None => Self::malformed(entry),
            },
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
            Key::USER_ID_STORAGE => match NonNull::new(entry.value[0] as *mut [u8; USER_ID_LEN]) {
                Some(storage) => Self::UserIdStorage(storage),
                None => Self::malformed(entry),
            },
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

    /// Reports `entry` as a known key whose payload cannot be used.
    ///
    /// Keeps the words as they arrived so the entry encodes back to itself and
    /// a consumer that can make sense of them still may.
    const fn malformed(entry: ConfigEntry) -> Self {
        Self::Malformed {
            key: entry.key,
            flags: entry.flags,
            value: entry.value,
        }
    }
}

impl From<Entry<'_>> for ConfigEntry {
    fn from(entry: Entry<'_>) -> Self {
        // Flags are not the entry's to decide: only the two catch-all variants
        // carry any, by having preserved what they were handed. A loader
        // marking an entry mandatory does so when it appends it to the list.
        match entry {
            Entry::LoaderInfo(text) => Self::new(
                Key::LOADER_INFO,
                [
                    text.map_or(0, |text| text.as_ptr() as u64),
                    text.map_or(0, |text| text.len() as u64),
                ],
            ),
            Entry::MainThreadHandle(handle) => {
                Self::new(Key::MAIN_THREAD_HANDLE, [u64::from(handle.to_raw()), 0])
            }
            Entry::ProcessHandle(handle) => {
                Self::new(Key::PROCESS_HANDLE, [u64::from(handle.to_raw()), 0])
            }
            Entry::NextLoadPath { path, argv } => Self::new(
                Key::NEXT_LOAD_PATH,
                [path.as_ptr() as u64, argv.as_ptr() as u64],
            ),
            Entry::OverrideHeap(heap) => Self::new(
                Key::OVERRIDE_HEAP,
                [heap.cast::<u8>().as_ptr() as u64, heap.len() as u64],
            ),
            Entry::OverrideService { name, handle } => Self::new(
                Key::OVERRIDE_SERVICE,
                [name.to_u64(), u64::from(handle.to_raw())],
            ),
            Entry::Argv(argv) => Self::new(Key::ARGV, [0, argv.as_ptr() as u64]),
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
                Self::new(Key::USER_ID_STORAGE, [storage.as_ptr() as u64, 0])
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
            Entry::Unknown { key, flags, value } | Entry::Malformed { key, flags, value } => {
                Self { key, flags, value }
            }
        }
    }
}

/// Marks the second word of a [`Key::HOS_VERSION`] entry as reporting a custom
/// firmware: `ATMOSPHR` read as little-endian ASCII.
const ATMOSPHERE_MAGIC: u64 = 0x41544d4f53504852;

/// How many bytes an account user id occupies, which is the size of the storage
/// a [`Key::USER_ID_STORAGE`] entry points at.
pub const USER_ID_LEN: usize = 16;

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
///
/// Flags survive decoding only on [`Entry::Unknown`] and [`Entry::Malformed`].
/// Every named variant drops them, because the flags exist to tell a reader
/// what to do about an entry it cannot act on, and a reader that got a named
/// variant can act on it.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryFlags(u32);

impl EntryFlags {
    /// Nothing beyond reading it.
    pub const NONE: Self = Self(0);
    /// The program must act on this entry or return to the loader.
    ///
    /// Read-time, and live on one branch only: the one where the reader has a
    /// key it cannot act on. A reader that recognises the key acts on it and
    /// the mark changes nothing, so the mark is worth setting on a key a reader
    /// might not know and worth nothing on a key every reader handles.
    ///
    /// It is dropped by [`Entry::decode`] for every key this crate names, so a
    /// consumer sees it only through [`Entry::Unknown`] or
    /// [`Entry::Malformed`]. See the crate docs for which entries it can reach
    /// and why skipping one is not a safe default.
    pub const MANDATORY: Self = Self(1 << 0);

    /// Names the flags the given word stands for.
    pub const fn new(flags: u32) -> Self {
        Self(flags)
    }

    /// Returns whether the loader requires this entry be acted on.
    ///
    /// Ask this where the answer can change what happens, which is on
    /// [`Entry::Unknown`] and [`Entry::Malformed`]: a `true` there means the
    /// program must return to the loader rather than carry on without the
    /// entry. Everywhere else the flags arrived as [`NONE`](Self::NONE),
    /// because decoding a key this crate names discards them.
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
