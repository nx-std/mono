//! # nx-hbabi
//!
//! The homebrew ABI: everything a loader and the program it starts must agree
//! on in advance, because at the moment of the handover neither can ask.
//!
//! There are two agreements, and this crate holds both.
//!
//! # Where the program goes: [`nro`]
//!
//! A program arrives as an NRO image, which is three segments and a zero-filled
//! tail. Which of them ends up executable, which read-only, and which writable
//! is fixed by the format: a loader that mapped them some other way would map a
//! program that cannot run. [`nro::map`] performs that placement and
//! [`MappedNro::unmap`](nro::MappedNro::unmap) reverses it, so a loader chooses
//! *which* program to run and not how one is laid out.
//!
//! # What the program is told: [`ConfigEntry`]
//!
//! Having mapped it, the loader jumps to the program's entry point with a
//! pointer to an array of [`ConfigEntry`] in the first argument register. Each
//! entry is a key, a flag word, and two payload words whose meaning the key
//! decides. The array ends at the entry whose key is [`Key::LOADER_INFO`],
//! which doubles as the terminator and as the loader's chance to name itself.
//!
//! Everything the program learns about how it was started - where its heap is,
//! what its command line says, which services the loader has already opened on
//! its behalf, which supervisor calls it is allowed to make - arrives through
//! that array, because at that moment nothing else has been set up to ask.
//!
//! # Why this is a crate and not a parser
//!
//! The array has two sides, and until now this workspace only had one: the
//! program's. A loader is the other side, and a loader that writes its entries
//! against a second, hand-written copy of the key table has an ABI defined in
//! two places that agree only as long as someone keeps checking.
//!
//! So the key table is stated once, in [`Entry`], and the two directions are the
//! two conversions on it:
//!
//! - [`Entry::decode`] - what a **program** does with what it was handed. Every
//!   key this crate knows becomes a variant naming the fields; every key it does
//!   not becomes [`Entry::Unknown`], which keeps enough to apply the mandatory
//!   rule below.
//! - `ConfigEntry::from(entry)` - what a **loader** does to say the same thing.
//!   It is the exact inverse, which is what makes a round trip through the wire
//!   format a no-op and what keeps the two sides from drifting.
//!
//! Neither side reaches for a key constant to do its job; both reach for the
//! variant, and the packing is written down once.
//!
//! The variants hold what the key already implies rather than the untyped words
//! it travels as: a thread handle where the key names the main thread, a slice
//! where the entry carries an address and a length. That makes a loader's side
//! ordinary safe Rust and leaves the program's side one `unsafe` - the promise
//! that the addresses it was handed are good - taken once at
//! [`decode`](Entry::decode) rather than at every use.
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
//! # What a loader cannot leave out
//!
//! Most of the format is a loader telling a program something it would
//! otherwise have to default. Three entries are not like that: the heap, the
//! main thread's handle and the process's own handle name things a program has
//! no way to obtain for itself. A process cannot ask the kernel for a real
//! handle to itself or to its main thread, and the pseudo-handles standing in
//! for them cannot be sent over IPC; the heap belongs to the loader because the
//! program runs inside a process the loader already carved one out of.
//!
//! So those three are not appended, they are type parameters on
//! [`ConfigListBuilder`], each starting at its own unsupplied marker and moved
//! to the supplied one by its own setter.
//! [`build`](ConfigListBuilder::build) exists only where all three are
//! supplied. Forgetting one is a compile error rather than a program that
//! starts and then behaves as though the loader had said nothing. The type
//! parameters are what enforce that, and they are the only thing that does: the
//! entries the setters write also carry
//! [`MANDATORY`](EntryFlags::MANDATORY), but on keys this well known that mark
//! is a statement of intent rather than a check, for the reasons below.
//!
//! The markers are per entry, one small module each: [`heap::Set`] stands only
//! in the heap parameter, so the three cannot be transposed, and a builder type
//! reads as [`heap::Set`], [`main_thread::Unset`] and so on rather than asking
//! the reader to count positions.
//!
//! # The mandatory flag
//!
//! An entry may be marked [mandatory](EntryFlags::MANDATORY), which means the
//! loader is not willing to run the program unless the program acts on it. A
//! program that does not recognise such a key must return to the loader instead
//! of continuing.
//!
//! The flag is written by a loader but it is a **read-time** instruction, and
//! it takes effect on exactly one branch: the one where the reader has a key it
//! cannot act on. A reader that handles the key acts on it, and the flag
//! changes nothing; a reader that does not handle it would otherwise skip it,
//! and the flag is what turns that skip into a refusal to run.
//!
//! So the flag is a forward-compatibility device. It is there for an older
//! program meeting a newer loader, and the failure it prevents is a silent one:
//! a loader that overrides the heap is saying the program's usual heap is not
//! there, and a program that skipped the key it did not know would run against
//! an address that is not mapped, faulting somewhere unrelated to the entry
//! that caused it. [`Key::OVERRIDE_SERVICE`] is the sharpest case, because
//! nothing faults at all: the program opens its own session and quietly
//! bypasses the interposition the loader set up.
//!
//! ## Which entries it can reach
//!
//! Only the ones a reader could fail to recognise. [`Entry::decode`] keeps
//! `flags` on [`Entry::Unknown`] and [`Entry::Malformed`] and nowhere else, so
//! a mandatory mark on a key this crate knows is dropped on the way in and no
//! consumer can branch on it. Marking a well-known key costs nothing and buys
//! nothing.
//!
//! It is also not preserved across a round trip. `ConfigEntry::from` writes
//! [`EntryFlags::NONE`] for every named variant, so a loader that decodes a
//! list and re-encodes it to pass on drops the mark from every key it
//! recognised. Only the two catch-all variants carry their flags back out.
//!
//! The three entries [`ConfigListBuilder`] requires are marked, and that mark
//! reaches only a reader that inspects raw [`ConfigEntry`] values rather than
//! decoding them: they are keys every reader of a list handles, so nothing that
//! goes through [`Entry::decode`] will ever see the bit.
//!
//! ## Acting on it
//!
//! This crate reports the condition and does not act on it: returning to the
//! loader means calling the address the loader left behind, which the runtime
//! holds and this crate does not. A consumer matches [`Entry::Unknown`] and
//! asks [`EntryFlags::is_mandatory`].
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

mod entry;
mod list;
pub mod nro;

pub use self::{
    entry::{
        ConfigEntry,
        Entry,
        EntryFlags,
        Key,
        USER_ID_LEN,
    },
    list::{
        ConfigEntries,
        ConfigList,
        ConfigListBuilder,
        MAX_APPENDED,
        MAX_ENTRIES,
        heap,
        main_thread,
        process,
    },
};
