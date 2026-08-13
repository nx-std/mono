//! # nx-std-env
//!
//! The process's environment, in the shape `std` gives it.
//!
//! This crate is the workspace's stand-in for `std::env`.
//!
//! ## What it is for
//!
//! The layers underneath speak in the terms the platform uses: `nx-sys-args`
//! holds arguments as bytes borrowed out of the loader's own buffer, and
//! `nx-sys-env` reaches the C library's environment in bytes under the lock
//! that orders those calls. Both are right for the layer that owns the
//! resource and wrong for a caller, who wants the vocabulary `std` uses.
//! Turning the one into the other is this crate's whole job, and it is the
//! division `std` makes: `std::sys` holds, `std::env` presents.
//!
//! ## Nothing here initializes anything
//!
//! There is no setup call, and adding one would be a mistake. `std::env` reads
//! through to whatever is underneath at the moment it is asked; the only part
//! of the platform layer `std` initializes eagerly is the argument store, and
//! that is the entry crate's job, done before anything here is reachable.
//!
//! ## The environment starts empty
//!
//! A Switch process is handed a command line and no environment block, so the
//! bindings a Unix program inherits from its parent have no counterpart here.
//! The mechanism is entirely present — the C library implements the POSIX
//! environment in full — and simply begins with nothing in it, so every
//! binding a program reads is one it, or something it linked, put there.
//!
//! ## What is absent, and why
//!
//! The surface is the part of `std::env` this workspace can answer honestly.
//! Absent so far:
//!
//! - `current_dir` and `set_current_dir`, until the working directory can be
//!   read back as well as set.
//! - `current_exe`, `home_dir` and `temp_dir`, which name things a Switch
//!   process either does not have or cannot ask for.
//! - `split_paths` and `join_paths`, which describe a `PATH` variable that
//!   nothing here sets.
//!
//! Each belongs here the moment its answer does.
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

extern crate alloc;

// Proves a global allocator exists for the owned types below; the umbrella still owns the
// singular `#[global_allocator]` registration at link time.
extern crate nx_alloc as _;

mod args;
pub mod consts;
mod vars;

pub use self::{
    args::{
        Args,
        ArgsOs,
        args,
        args_os,
    },
    vars::{
        VarError,
        Vars,
        VarsOs,
        remove_var,
        set_var,
        var,
        var_os,
        vars,
        vars_os,
    },
};
