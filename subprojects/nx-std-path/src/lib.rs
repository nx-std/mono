//! # nx-std-path
//!
//! Platform strings and filesystem paths, in the shape `std` gives them.
//!
//! This crate is the workspace's stand-in for `std::ffi::OsStr` and `std::path::Path`. It exists
//! because the crates above it pass paths around, and the type they were passing was
//! [`core::ffi::CStr`]: the form the C standard library happens to deliver a path in, carried all
//! the way down to the code that builds an IPC command. That is backwards. A nul terminator is a
//! property of one boundary, not of a path.
//!
//! ## The layering, and why it is the one `std` uses
//!
//! [`OsStr`] is a byte string the operating system will accept, before anything has decided it is
//! text. [`Path`] is an [`OsStr`] that is read as a filesystem location. Owned counterparts,
//! [`OsString`] and [`PathBuf`], sit beside each of them.
//!
//! Horizon's substrate is newlib, and its paths are `/`-separated byte strings, so the layering
//! this crate implements is the Unix one: an [`OsStr`] is `[u8]` with no encoding requirement, and
//! `/` is the separator. That matters beyond taste. The `std` port this workspace is working
//! towards consumes a platform layer that speaks `&Path` and converts to a nul-terminated string
//! at the syscall itself; a layer that speaks `CStr` throughout is one `std` would have to be
//! taught about.
//!
//! ## Bytes in, bytes out
//!
//! Nothing here validates UTF-8. A path arrives as bytes, is carried as bytes, and is written back
//! out as bytes, so a name the encoding rules do not describe reaches the filesystem exactly as its
//! caller wrote it rather than being refused by a layer with no business judging it. Code that
//! genuinely needs text asks for it with [`OsStr::to_str`] and handles the `None`.
//!
//! ## What is here, and what is not
//!
//! The surface is the part of `std` this workspace calls, and every item on it behaves as its `std`
//! namesake does. Absent so far, because nothing needs them yet: the `Components` iterator and
//! everything `std` derives from it, which is `parent`, `file_name`, `file_stem`, `extension`,
//! `starts_with` and `strip_prefix`. They belong here the moment a caller appears, and they belong
//! here built on `Components` rather than on a scan for the last separator: the two answer
//! differently on a trailing slash, on a repeated separator, and on a path ending in `..`, and an
//! approximation that is right on the paths someone happened to test is worse than an absence.
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

extern crate alloc;

// Proves a global allocator exists for the owned types below; the umbrella still owns the
// singular `#[global_allocator]` registration at link time.
extern crate nx_alloc as _;

mod os_str;
mod path;

pub use self::{
    os_str::{
        OsStr,
        OsString,
    },
    path::{
        Display,
        MAIN_SEPARATOR,
        Path,
        PathBuf,
    },
};
