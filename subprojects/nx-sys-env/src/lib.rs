//! # nx-sys-env
//!
//! The process's environment variables.
//!
//! This crate is the workspace's stand-in for `std::sys::env`.
//!
//! ## The environment is this crate's, and it starts empty
//!
//! A Switch process is handed a command line and nothing else. There is no
//! block to inherit, no parent to inherit it from, and no loader key that
//! carries one, so a process begins with an empty environment and every
//! binding in it is one the process put there.
//!
//! That makes the environment something this workspace owns outright rather
//! than something it reads out of a C library, and it is why the store here is
//! a Rust one. The C library ships an environment of its own, and building on
//! it would mean adopting a second copy of state this crate is meant to
//! provide; the direction is the reverse, as it is everywhere else here.
//!
//! With the `ffi` feature the C surface is redirected here too, so the two
//! environments become one and a binding made from either side is visible to
//! both.
//!
//! ## Why nothing outside the C surface is `unsafe`
//!
//! `std::env::set_var` is an `unsafe fn`, and the reason is not the assignment:
//! it is that the C library keeps its environment in a bare global with no lock
//! on it, so an assignment can free the block another thread is walking, and no
//! caller of the C API can be stopped from doing so.
//!
//! Here the environment is behind a lock and reachable no other way. Readers
//! share it, a writer excludes them, and the C entry points go through the same
//! lock. The hazard that makes the `std` call `unsafe` does not exist, so [`set`]
//! and [`unset`] are ordinary safe functions.
//!
//! ## Bytes, not text
//!
//! A name and a value are byte strings. Nothing here validates UTF-8, for the
//! reason nothing does at this layer: the vocabulary belongs to the crate above.
//!
//! Two byte sequences are still refused, and both because of the C surface this
//! store serves. A name may not be empty or hold `=`, which is what separates a
//! name from a value; and neither may hold a nul, which is what terminates one
//! for a C caller.
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

extern crate alloc;

// Proves a global allocator exists for the owned types below; the umbrella still owns the
// singular `#[global_allocator]` registration at link time.
extern crate nx_alloc as _;

mod environment;
#[cfg(feature = "ffi")]
pub mod ffi;

pub use self::environment::{
    SetError,
    Vars,
    get,
    set,
    unset,
    vars,
};
