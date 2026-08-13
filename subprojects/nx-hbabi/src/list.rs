//! The entry list, from both ends: walking one a loader handed over, and
//! assembling one to hand over.

use core::ptr::NonNull;

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

/// How many entries a loader may append before
/// [`build`](ConfigListBuilder::build) adds the terminator.
///
/// One short of [`MAX_ENTRIES`], because the terminator is an entry and its
/// slot is reserved from the start. That is what lets `build` be infallible:
/// there is no arrangement of appends that leaves a list unterminated.
pub const MAX_APPENDED: usize = MAX_ENTRIES - 1;

/// Walks the entry list a loader handed over.
///
/// Yields decoded [`Entry`] values and stops at the terminator, or at
/// [`MAX_ENTRIES`] if the list has none. Entries are read one at a time rather
/// than as a slice up front, because how many there are is not known until the
/// terminator is reached and a slice covering the maximum would read past a
/// short list.
pub struct ConfigEntries {
    next: NonNull<ConfigEntry>,
    read: usize,
    done: bool,
}

impl ConfigEntries {
    /// Starts walking the list beginning at `ptr`.
    ///
    /// # Safety
    ///
    /// `ptr` must be the start of an entry list this process may read, either
    /// terminated by a [`Key::LOADER_INFO`] entry or at least [`MAX_ENTRIES`]
    /// entries long.
    pub const unsafe fn from_ptr(ptr: NonNull<ConfigEntry>) -> Self {
        Self {
            next: ptr,
            read: 0,
            done: false,
        }
    }
}

impl Iterator for ConfigEntries {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.read >= MAX_ENTRIES {
            return None;
        }

        // SAFETY: the caller of `from_ptr` vouched for a list that is either
        // terminated or `MAX_ENTRIES` long, and both bounds are checked above,
        // so this entry is within it.
        let raw = unsafe { self.next.read() };

        // SAFETY: the entry just read is within the list, so the list has
        // either another entry after it or ends here, and ending here is what
        // the terminator check below detects before the pointer is read again.
        self.next = unsafe { self.next.add(1) };
        self.read += 1;

        let entry = Entry::from(raw);
        if matches!(entry, Entry::LoaderInfo { .. }) {
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
#[derive(Debug, Clone)]
pub struct ConfigList {
    entries: [ConfigEntry; MAX_ENTRIES],
    len: usize,
}

impl ConfigList {
    /// Starts assembling a list.
    pub fn builder() -> ConfigListBuilder {
        ConfigListBuilder {
            entries: [PLACEHOLDER; MAX_ENTRIES],
            len: 0,
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
/// Entries are appended in order and the terminator is written by
/// [`build`](Self::build), so a list cannot be handed over unterminated.
#[derive(Debug, Clone)]
pub struct ConfigListBuilder {
    entries: [ConfigEntry; MAX_ENTRIES],
    len: usize,
}

impl ConfigListBuilder {
    /// Appends an entry the program may skip if it does not know the key.
    ///
    /// # Panics
    ///
    /// Panics in debug builds once [`MAX_APPENDED`] entries are appended.
    pub fn push(self, entry: Entry) -> Self {
        self.push_encoded(ConfigEntry::from(entry))
    }

    /// Appends an entry the program must act on or return to the loader over.
    ///
    /// Mark an entry this way when running without it would be worse than not
    /// running: an overridden heap the program does not move to, a service
    /// session it does not use in place of opening its own. See the crate docs.
    ///
    /// # Panics
    ///
    /// Panics in debug builds once [`MAX_APPENDED`] entries are appended.
    pub fn push_mandatory(self, entry: Entry) -> Self {
        self.push_encoded(ConfigEntry::from(entry).with_flags(EntryFlags::MANDATORY))
    }

    /// Terminates the list, naming the loader in the entry that does it.
    ///
    /// `info` is free-form text a program may show; pass an empty slice to name
    /// nothing. It is borrowed for as long as the list is, because the program
    /// reads it through the pointer this entry carries rather than a copy.
    ///
    /// Terminating cannot fail for want of room: [`MAX_APPENDED`] holds the
    /// last slot back for exactly this entry.
    pub fn build(self, info: &'static [u8]) -> ConfigList {
        // An empty slice still has a non-null address, so absence is decided on
        // the length rather than on what `as_ptr` hands back. Otherwise a
        // loader that named nothing would point a program at a dangling
        // address and tell it the text was zero bytes long.
        let text = if info.is_empty() {
            None
        } else {
            NonNull::new(info.as_ptr().cast_mut()).map(NonNull::cast)
        };
        let terminated = self.append(ConfigEntry::from(Entry::LoaderInfo {
            text,
            len: info.len() as u64,
        }));

        ConfigList {
            entries: terminated.entries,
            len: terminated.len,
        }
    }

    /// Appends one already-encoded entry, keeping the terminator's slot free.
    ///
    /// Over the cap the entry is dropped rather than taking the slot the
    /// terminator needs. A loader that appends too much loses an entry, which
    /// is a program starting under-informed; letting the write through would
    /// lose the terminator instead, which is a program reading past the end of
    /// the list.
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

    /// Writes one entry into the next slot, terminator included.
    fn append(mut self, entry: ConfigEntry) -> Self {
        self.entries[self.len] = entry;
        self.len += 1;
        self
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
