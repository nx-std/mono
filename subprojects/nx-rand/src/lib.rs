//! # nx-rand
//!
//! Horizon's entropy source, wired into the seam the `rand` ecosystem ports through.
//!
//! # What this crate is
//!
//! It is not a random-number API. `rand` already is one, and a second one in its shape would only
//! be something callers have to learn. This crate supplies the one thing `rand` cannot bring with
//! it: where the bytes come from on this platform. With the `getrandom-backend` feature on it
//! defines `__getrandom_v03_custom`, the symbol `getrandom` resolves entropy through, and every
//! crate in the workspace can then write `rand::rngs::SysRng` or `StdRng::from_os_rng()` with no
//! dependency on this crate at all.
//!
//! The C surface is the same source under its other name: `randomGet` and `randomGet64`, behind
//! the `ffi` feature.
//!
//! # Where the bytes come from
//!
//! The kernel assigns a process 256 bits of entropy and offers no way to ask for more, so
//! [`entropy`] expands that seed with a ChaCha20 generator shared by the whole process. A caller
//! that wants an independent stream seeds one from here rather than reaching for the kernel again,
//! which is what `rand`'s own generators do when they take their seed from `SysRng`.
//!
//! One consequence is worth stating plainly: everything the process ever emits is a function of
//! those 256 bits. `rand`'s hosted `ThreadRng` periodically reseeds from an OS source that keeps
//! absorbing new entropy, which bounds what recovering a generator's state reveals. Horizon
//! exposes no such stream below the service layer, so there is nothing to reseed from here and
//! that bound does not exist. It is also why `ThreadRng` itself is out of reach: it is gated on
//! `std`, which this target does not have.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

pub mod entropy;

#[cfg(feature = "ffi")]
pub mod ffi;
#[cfg(feature = "getrandom-backend")]
mod getrandom;
