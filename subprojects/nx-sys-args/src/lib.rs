//! # nx-sys-args
//!
//! The process command line: the store that holds it, the scanner that splits
//! it, and the iterator a caller reads it back through.
//!
//! This crate is the workspace's stand-in for `std::sys::args`.
//!
//! ## Why the store is here and not in the runtime
//!
//! Only the runtime can *read* the command line. A homebrew NRO receives it
//! through the loader's configuration block and an NSO through the
//! page-aligned `__argdata__` region; neither source is visible from below.
//! That makes an entry crate the injector, and nothing else can be.
//!
//! It does not make the entry crate the owner. `std` splits the two: `std::rt`
//! takes `argc`/`argv` and hands them straight to `std::sys::args`, which
//! holds them, and `std::env::args` reads them back from there. Windows makes
//! the same split with none of the same pieces, scanning a single command-line
//! string inside `std::sys::args` and storing nothing at all. What survives
//! every platform is where the answer lives, not who fetches it.
//!
//! Keeping the store here buys what it buys `std`: a caller reads [`args`]
//! through a dependency that points down, rather than up at the entry crate
//! that happens to have started the program. The runtime is the last crate in
//! the graph, so anything owned there is out of reach of everything else.
//!
//! ## Nothing is copied and nothing is allocated
//!
//! The argument string the loader hands over lives for the whole process, so an
//! argument is a subslice of it rather than a copy. Scanning writes a nul over
//! each separator it consumes, which is what lets the same bytes serve a C
//! caller through [`ffi`] without a second buffer, and the ranges it records
//! live in a fixed-size table.
//!
//! No allocator appears in this crate's dependency graph, so heap-freedom is
//! the compiler's to enforce rather than a property someone has to keep
//! remembering. That is what lets the command line be installed before the heap
//! exists, which is why an entry crate's argument setup needs no ordering
//! promise from its caller.
//!
//! What it costs is a bound: [`MAX_ARGS`] arguments, past which the rest are
//! dropped. A growable table would need the allocator this crate exists without.
//!
//! ## Bytes, not text
//!
//! An argument is a byte string. Nothing here validates UTF-8, so a command
//! line the encoding rules do not describe reaches its caller as the loader
//! wrote it instead of being dropped by a layer with no business judging it.
//!
//! Bytes are also as far as this layer goes. Giving arguments out as the
//! `OsString` `std` uses would mean depending on the crate that defines it,
//! which sits above this one; the layer that owns that vocabulary is the one
//! to apply it, and the conversion costs nothing, since `OsString` is built
//! from exactly the bytes [`args`] yields.
//!
//! ## How the command line is split
//!
//! Arguments are separated by ASCII whitespace, and a `"` pair quotes one whole
//! argument. Quoting delimits an argument rather than toggling within one, so
//! that every argument is a contiguous run of the loader's buffer and can be
//! named by a range instead of assembled into a new one.
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

mod command_line;

#[cfg(feature = "ffi")]
pub mod ffi;

pub use self::command_line::{
    Args,
    MAX_ARGS,
    args,
    setup_from,
};
