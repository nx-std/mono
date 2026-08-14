//! The entry list, from both ends: walking one a loader handed over, and
//! assembling one to hand over.

use core::{
    marker::PhantomData,
    ptr::NonNull,
};

use nx_svc::{
    process::Handle as ProcessHandle,
    thread::Handle as ThreadHandle,
};

use super::entry::{
    ConfigEntry,
    Entry,
    EntryFlags,
    Key,
};

/// How many entries a list may hold, terminator included.
///
/// The format sets no limit, so this is a bound on what can sensibly appear:
/// each single-instance key once, a service override per name the service
/// manager admits, and the terminator. A program walking a list stops here
/// rather than reading forever if the terminator is missing, and a loader
/// building one cannot exceed what a program will read.
pub const MAX_ENTRIES: usize = 48;

/// How many entries a loader may append with [`push`](ConfigListBuilder::push)
/// or [`push_mandatory`](ConfigListBuilder::push_mandatory).
///
/// Short of [`MAX_ENTRIES`] by the four slots held back for the entries the
/// builder writes itself: the three a loader must supply, and the terminator.
/// That is what lets those writes be
/// infallible: no run of pushes can take a slot one of them needs, so there is
/// no arrangement of calls that drops a required entry or leaves a list
/// unterminated.
pub const MAX_APPENDED: usize = MAX_ENTRIES - RESERVED;

/// How many slots are held back from [`MAX_APPENDED`].
///
/// The three entries only a loader can supply, plus the terminator. Each is
/// written at most once: the terminator by [`build`](ConfigListBuilder::build),
/// and the other three by setters the type system allows one call each.
const RESERVED: usize = 4;

/// Walks the entry list a loader handed over.
///
/// Yields decoded [`Entry`] values and stops at the terminator, or at
/// [`MAX_ENTRIES`] if the list has none. Entries are read one at a time rather
/// than as a slice up front, because how many there are is not known until the
/// terminator is reached and a slice covering the maximum would read past a
/// short list.
/// `'a` is how long the memory the entries name stays readable, which the
/// caller of [`from_ptr`](Self::from_ptr) chooses and vouches for once on
/// behalf of every entry the walk yields.
pub struct ConfigEntries<'a> {
    next: NonNull<ConfigEntry>,
    read: usize,
    done: bool,
    list: PhantomData<&'a [ConfigEntry]>,
}

impl<'a> ConfigEntries<'a> {
    /// Starts walking the list beginning at `ptr`.
    ///
    /// # Safety
    ///
    /// `ptr` must be the start of an entry list this process may read, either
    /// terminated by a [`Key::LOADER_INFO`] entry or at least [`MAX_ENTRIES`]
    /// entries long.
    ///
    /// Every address the entries carry must additionally satisfy
    /// [`Entry::decode`] for `'a`, since that is what each one is read through.
    pub const unsafe fn from_ptr(ptr: NonNull<ConfigEntry>) -> Self {
        Self {
            next: ptr,
            read: 0,
            done: false,
            list: PhantomData,
        }
    }
}

impl<'a> Iterator for ConfigEntries<'a> {
    type Item = Entry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.read >= MAX_ENTRIES {
            return None;
        }

        // SAFETY: the caller of `from_ptr` vouched for a list that is either
        // terminated or `MAX_ENTRIES` long, and both bounds are checked above,
        // so this entry is within it.
        let raw = unsafe { self.next.read() };

        // SAFETY: the caller of `from_ptr` vouched for every address in the
        // list naming memory readable for `'a`, which is what decoding this
        // entry into borrows of that lifetime requires.
        let entry = unsafe { Entry::decode(raw) };

        // SAFETY: the entry just read is within the list, so the list has
        // either another entry after it or ends here, and ending here is what
        // the terminator check below detects before the pointer is read again.
        self.next = unsafe { self.next.add(1) };
        self.read += 1;

        if matches!(entry, Entry::LoaderInfo(_)) {
            self.done = true;
        }

        Some(entry)
    }
}

/// An entry list a loader has assembled, ready to hand to a program.
///
/// Held by value rather than written into caller memory because the loader has
/// to keep it alive across the jump to the program: the program reads the list
/// during its own startup, long after the call that passed the pointer.
///
/// `'a` is how long the memory the appended entries named stays alive. The
/// encoded list holds those as bare addresses, so the lifetime is what keeps a
/// list from outliving the buffers it points into.
#[derive(Debug, Clone)]
pub struct ConfigList<'a> {
    entries: [ConfigEntry; MAX_ENTRIES],
    len: usize,
    // Encoding turned the entries' borrows into plain addresses, so no field
    // above still ties this list to the buffers it points into. This is what
    // puts that back: without it a list could outlive its own loader-info text
    // and `as_ptr` would hand a program a dangling address.
    named: PhantomData<&'a [u8]>,
}

impl<'a> ConfigList<'a> {
    /// Starts assembling a list, with none of the required entries supplied.
    pub fn builder() -> ConfigListBuilder<'a, heap::Unset, main_thread::Unset, process::Unset> {
        ConfigListBuilder {
            entries: [PLACEHOLDER; MAX_ENTRIES],
            len: 0,
            named: PhantomData,
            supplied: PhantomData,
        }
    }

    /// Returns the entries, terminator included.
    pub fn as_slice(&self) -> &[ConfigEntry] {
        &self.entries[..self.len]
    }

    /// Returns the address to pass the program as its first argument.
    ///
    /// The list must outlive the program's startup, which reads through this
    /// pointer after the call that handed it over has returned.
    pub fn as_ptr(&self) -> *const ConfigEntry {
        self.entries.as_ptr()
    }
}

/// Assembles a [`ConfigList`].
///
/// Optional entries are appended in any order and the terminator is written by
/// [`build`](Self::build), so a list cannot be handed over unterminated.
///
/// # The three a loader must supply
///
/// Three entries name things a program cannot obtain for itself, so a list
/// without them is one the program starts wrong on rather than one it starts
/// under-informed on. They are type parameters instead of appends, in this
/// order:
///
/// - `H`, the heap the program runs on, from [`heap`](Self::heap)
/// - `T`, the main thread's handle, from [`main_thread`](Self::main_thread)
/// - `P`, the process's own handle, from [`process`](Self::process)
///
/// Each starts unsupplied, its setter moves it to supplied, and
/// [`build`](Self::build) exists only where all three are supplied. A loader
/// that forgets one does not hand over a list a program limps along on; it
/// fails to compile. A setter is reachable only while its own entry is
/// unsupplied, so supplying one twice fails to compile as well.
///
/// Why these three and no others: a process cannot ask the kernel for a real
/// handle to itself or to its own main thread, and the pseudo-handles that
/// stand in for them cannot be sent over IPC. The heap is the loader's because
/// the program is running inside a process the loader already carved one out
/// of. Everything else in the format is either something the program can
/// default or something it can work out.
///
/// # One marker per entry, not one shared pair
///
/// Each parameter has its own pair of markers and its own trait, so
/// [`heap::Set`] fits nowhere but `H`. A single shared `Set`/`Unset` pair would
/// leave the three parameters interchangeable, and two things would follow.
///
/// A reader would have to know that the second parameter is the main thread,
/// since nothing in `<Set, Unset, Set>` says so. And a setter whose return type
/// named a marker for the wrong parameter would still compile, so
/// [`heap`](Self::heap) could mark the main thread as supplied and nothing
/// would object. Distinct types make that one a compile error at the setter
/// rather than a wrong list at run time, which matters here because the crate
/// builds with neither unit tests nor doctests and has no other place to catch
/// it.
///
/// What they do not catch is a setter that leaves its own parameter alone and
/// advances another: every marker still sits in a parameter that accepts it.
/// Each setter therefore states its whole transition in one place, its return
/// type and the value it returns side by side, rather than delegating the move
/// to a shared helper generic enough to perform any of them.
#[derive(Debug, Clone)]
pub struct ConfigListBuilder<'a, H: heap::State, T: main_thread::State, P: process::State> {
    entries: [ConfigEntry; MAX_ENTRIES],
    len: usize,
    // Entries are encoded as they are appended, so the builder erases their
    // borrows exactly as the finished list does and needs the same stand-in.
    named: PhantomData<&'a [u8]>,
    // One field for all three positions rather than one each, so a setter
    // rewrites a single field instead of three.
    supplied: PhantomData<(H, T, P)>,
}

impl<'a, T: main_thread::State, P: process::State> ConfigListBuilder<'a, heap::Unset, T, P> {
    /// Supplies the heap the program runs on in place of asking the kernel.
    ///
    /// The region is uninitialised memory on its way to an allocator, which is
    /// why it is a pointer to a length rather than a slice: nothing may read it
    /// as bytes or assume it is unaliased.
    pub fn heap(self, heap: NonNull<[u8]>) -> ConfigListBuilder<'a, heap::Set, T, P> {
        let written = self
            .append(ConfigEntry::from(Entry::OverrideHeap(heap)).with_flags(EntryFlags::MANDATORY));

        ConfigListBuilder {
            entries: written.entries,
            len: written.len,
            named: PhantomData,
            supplied: PhantomData,
        }
    }
}

impl<'a, H: heap::State, P: process::State> ConfigListBuilder<'a, H, main_thread::Unset, P> {
    /// Supplies the handle naming the process's main thread.
    pub fn main_thread(
        self,
        handle: ThreadHandle,
    ) -> ConfigListBuilder<'a, H, main_thread::Set, P> {
        let written = self.append(
            ConfigEntry::from(Entry::MainThreadHandle(handle)).with_flags(EntryFlags::MANDATORY),
        );

        ConfigListBuilder {
            entries: written.entries,
            len: written.len,
            named: PhantomData,
            supplied: PhantomData,
        }
    }
}

impl<'a, H: heap::State, T: main_thread::State> ConfigListBuilder<'a, H, T, process::Unset> {
    /// Supplies the handle naming the process itself.
    pub fn process(self, handle: ProcessHandle) -> ConfigListBuilder<'a, H, T, process::Set> {
        let written = self.append(
            ConfigEntry::from(Entry::ProcessHandle(handle)).with_flags(EntryFlags::MANDATORY),
        );

        ConfigListBuilder {
            entries: written.entries,
            len: written.len,
            named: PhantomData,
            supplied: PhantomData,
        }
    }
}

impl<'a> ConfigListBuilder<'a, heap::Set, main_thread::Set, process::Set> {
    /// Terminates the list, naming the loader in the entry that does it.
    ///
    /// `info` is free-form text a program may show; pass an empty slice to name
    /// nothing. It is borrowed for as long as the list is, because the program
    /// reads it through the pointer this entry carries rather than a copy.
    ///
    /// Terminating cannot fail for want of room: one of the slots held back
    /// from [`MAX_APPENDED`] is reserved for exactly this entry.
    pub fn build(self, info: &'a [u8]) -> ConfigList<'a> {
        // An empty slice still has a non-null address, so absence is decided on
        // the length rather than on what the slice's pointer happens to be.
        // Otherwise a loader that named nothing would point a program at a
        // dangling address and tell it the text was zero bytes long.
        let text = (!info.is_empty()).then_some(info);
        let terminated = self.append(ConfigEntry::from(Entry::LoaderInfo(text)));

        ConfigList {
            entries: terminated.entries,
            len: terminated.len,
            named: PhantomData,
        }
    }
}

impl<'a, H: heap::State, T: main_thread::State, P: process::State> ConfigListBuilder<'a, H, T, P> {
    /// Appends an entry the program may skip if it does not know the key.
    ///
    /// # Panics
    ///
    /// Panics in debug builds once [`MAX_APPENDED`] entries are appended.
    pub fn push(self, entry: Entry<'a>) -> Self {
        self.push_encoded(ConfigEntry::from(entry))
    }

    /// Appends an entry the program must act on or return to the loader over.
    ///
    /// Mark an entry this way when running without it would be worse than not
    /// running, and when a reader might not recognise the key: the mark is
    /// read-time and only reaches the branch where the reader cannot act on the
    /// entry. A service override is the case that pays, since a program that
    /// skips it opens its own session and bypasses the interposition without
    /// anything faulting. See the crate docs for what the mark can and cannot
    /// reach.
    ///
    /// # Panics
    ///
    /// Panics in debug builds once [`MAX_APPENDED`] entries are appended.
    pub fn push_mandatory(self, entry: Entry<'a>) -> Self {
        self.push_encoded(ConfigEntry::from(entry).with_flags(EntryFlags::MANDATORY))
    }

    /// Appends one already-encoded entry, keeping the reserved slots free.
    ///
    /// Over the cap the entry is dropped rather than taking a slot a required
    /// entry or the terminator needs. A loader that appends too much loses an
    /// optional entry, which is a program starting under-informed; letting the
    /// write through would lose one of the reserved entries instead, which is a
    /// program starting wrong or reading past the end of the list.
    ///
    /// # Panics
    ///
    /// Panics in debug builds once [`MAX_APPENDED`] entries are appended.
    fn push_encoded(self, entry: ConfigEntry) -> Self {
        debug_assert!(self.len < MAX_APPENDED, "config entry list is full");
        if self.len < MAX_APPENDED {
            self.append(entry)
        } else {
            self
        }
    }

    /// Writes one entry into the next slot, reserved entries included.
    fn append(mut self, entry: ConfigEntry) -> Self {
        self.entries[self.len] = entry;
        self.len += 1;
        self
    }
}

/// Whether the heap entry has been supplied to a [`ConfigListBuilder`].
///
/// One module per required entry, each with its own markers and its own trait,
/// so a marker stands in one parameter and nowhere else. See the
/// [builder](ConfigListBuilder#one-marker-per-entry-not-one-shared-pair) for
/// what that rules out and what it does not.
pub mod heap {
    /// Whether the heap entry has been supplied.
    ///
    /// Sealed: [`Set`] and [`Unset`] are the only two answers, and a third
    /// would be a builder state neither the setters nor
    /// [`build`](super::ConfigListBuilder::build) know what to do with.
    pub trait State: _priv::Sealed {}

    /// The heap entry has been supplied.
    ///
    /// A marker: no value of it is ever constructed.
    #[derive(Debug, Clone, Copy)]
    pub struct Set;

    /// The heap entry has not been supplied yet.
    ///
    /// A marker: no value of it is ever constructed.
    #[derive(Debug, Clone, Copy)]
    pub struct Unset;

    impl State for Set {}

    impl State for Unset {}

    mod _priv {
        pub trait Sealed {}

        impl Sealed for super::Set {}

        impl Sealed for super::Unset {}
    }
}

/// Whether the main-thread handle has been supplied to a
/// [`ConfigListBuilder`].
pub mod main_thread {
    /// Whether the main-thread handle has been supplied.
    ///
    /// Sealed, on the same terms as [`heap::State`](super::heap::State).
    pub trait State: _priv::Sealed {}

    /// The main-thread handle has been supplied.
    ///
    /// A marker: no value of it is ever constructed.
    #[derive(Debug, Clone, Copy)]
    pub struct Set;

    /// The main-thread handle has not been supplied yet.
    ///
    /// A marker: no value of it is ever constructed.
    #[derive(Debug, Clone, Copy)]
    pub struct Unset;

    impl State for Set {}

    impl State for Unset {}

    mod _priv {
        pub trait Sealed {}

        impl Sealed for super::Set {}

        impl Sealed for super::Unset {}
    }
}

/// Whether the process handle has been supplied to a [`ConfigListBuilder`].
pub mod process {
    /// Whether the process handle has been supplied.
    ///
    /// Sealed, on the same terms as [`heap::State`](super::heap::State).
    pub trait State: _priv::Sealed {}

    /// The process handle has been supplied.
    ///
    /// A marker: no value of it is ever constructed.
    #[derive(Debug, Clone, Copy)]
    pub struct Set;

    /// The process handle has not been supplied yet.
    ///
    /// A marker: no value of it is ever constructed.
    #[derive(Debug, Clone, Copy)]
    pub struct Unset;

    impl State for Set {}

    impl State for Unset {}

    mod _priv {
        pub trait Sealed {}

        impl Sealed for super::Set {}

        impl Sealed for super::Unset {}
    }
}

/// Fills the slots of a list under construction that no entry has been written
/// into yet.
///
/// The array is fixed-size, so every slot holds something from the start. This
/// is never handed over: `len` bounds what a built list exposes, and the
/// terminator sits inside that bound.
const PLACEHOLDER: ConfigEntry = ConfigEntry {
    key: Key::LOADER_INFO,
    flags: EntryFlags::NONE,
    value: [0, 0],
};
