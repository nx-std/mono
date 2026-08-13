//! # nx-hbabi
//!
//! The homebrew ABI: the handover a loader performs when it starts a program it
//! has just mapped, and the only thing the two sides agree on in advance.
//!
//! A loader maps a program, then jumps to its entry point with a pointer to an
//! array of [`ConfigEntry`] in the first argument register. Each entry is a key,
//! a flag word, and two payload words whose meaning the key decides. The array
//! ends at the entry whose key is [`Key::LOADER_INFO`], which doubles as the
//! terminator and as the loader's chance to name itself.
//!
//! That is the whole format. Everything the program learns about how it was
//! started - where its heap is, what its command line says, which services the
//! loader has already opened on its behalf, which supervisor calls it is allowed
//! to make - arrives through it, because at that moment nothing else has been
//! set up to ask.
//!
//! # Why this is a crate and not a parser
//!
//! The format has two sides, and until now this workspace only had one: the
//! program's. A loader is the other side, and a loader that writes its entries
//! against a second, hand-written copy of the key table has an ABI defined in
//! two places that agree only as long as someone keeps checking.
//!
//! So the key table is stated once, in [`Entry`], and the two directions are the
//! two conversions on it:
//!
//! - `Entry::from(config_entry)` - what a **program** does with what it was
//!   handed. Every key this crate knows becomes a variant naming the fields;
//!   every key it does not becomes [`Entry::Unknown`], which keeps enough to
//!   apply the mandatory rule below.
//! - `ConfigEntry::from(entry)` - what a **loader** does to say the same thing.
//!   It is the exact inverse, which is what makes a round trip through the wire
//!   format a no-op and what keeps the two sides from drifting.
//!
//! Neither side reaches for a key constant to do its job; both reach for the
//! variant, and the packing is written down once.
//!
//! # Reading a list, writing a list
//!
//! [`ConfigEntries`] walks a list a loader handed over, and [`ConfigList`] is
//! one a loader has built. They are separate types rather than one that does
//! both, because a process only ever does one of the two: a program is handed
//! a list it did not write, and a loader writes a list it will not read.
//!
//! [`ConfigList`] is built through [`ConfigListBuilder`], which appends the
//! terminating loader-info entry in [`build`](ConfigListBuilder::build) rather
//! than trusting the caller to remember it. A list without a terminator is not
//! a shorter list, it is a program reading whatever follows it in memory.
//!
//! # The mandatory flag
//!
//! An entry may be marked [mandatory](EntryFlags::MANDATORY), which means the
//! loader is not willing to run the program unless the program acts on it. A
//! program that does not recognise such a key must return to the loader instead
//! of continuing.
//!
//! The rule exists because the alternative failure is silent. A loader that
//! overrides the heap and marks the entry mandatory is saying the program's
//! usual heap is not there; a program that skipped the key it did not know
//! would run against an address that is not mapped, and the fault would surface
//! somewhere unrelated to the entry that caused it.
//!
//! This crate reports the condition and does not act on it: returning to the
//! loader means calling the address the loader left behind, which the runtime
//! holds and this crate does not. A consumer matches [`Entry::Unknown`] and
//! asks [`EntryFlags::is_mandatory`].
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

mod entry;
mod list;

pub use self::{
    entry::{
        ConfigEntry,
        Entry,
        EntryFlags,
        Key,
    },
    list::{
        ConfigEntries,
        ConfigList,
        ConfigListBuilder,
        MAX_APPENDED,
        MAX_ENTRIES,
    },
};
