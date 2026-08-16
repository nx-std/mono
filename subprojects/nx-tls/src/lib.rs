//! # nx-tls
//!
//! The TLS stack's side of the C surface: what makes a socket become a TLS connection.
//!
//! This crate is to [`nx_service_ssl`] what `nx-net` is to `nx-service-sfdnsres`. That crate speaks
//! to the `ssl` service and stops there; this one owns the C symbols a program calls, and the work
//! that has to happen around a command before the service sees it.
//!
//! ## Why it sits above the runtime
//!
//! Every other crate holding a service's C surface sits *below* `nx-rt-core`, and pays for it: the
//! symbols needing something the runtime holds, a service-manager session or the firmware
//! version, get stranded in the runtime away from the surface they belong to. `nx-net` is split that way
//! today, with five of its resolver symbols defined in `nx-rt-core`.
//!
//! This crate takes the dependency the other direction instead. It depends on `nx-rt-core`, so it
//! reads the running firmware itself and keeps every one of its symbols, including the one gated on
//! `[16.0.0]`. A version gate is a reason to write a check, not a reason to move a function to
//! another crate.
//!
//! The cost is that `nx-rt-core` must never depend on this crate, which is what keeps the graph
//! acyclic. Nothing here is reached from the runtime; a program links this crate because it wants
//! TLS, and the linker script binds the symbols.
//!
//! ## What it holds
//!
//! The whole of upstream's `ssl` client: bringing the service up, contexts, connections,
//! handshakes, certificate handling, and the three socket hand-offs that gave the crate its first
//! reason to exist. All of it sits under [`ffi`], because all of it exists for the C boundary:
//! this crate exports no Rust API, and [`nx_service_ssl`] is what a Rust caller wants instead.
//!
//! That is also why the process-wide service connection needs no `extern-state` treatment. A
//! second static library cannot reach it, since there is nothing here to reach it *through*, and
//! the C ABI is single by construction.
//!
//! ## The system service variant is not selectable
//!
//! Upstream lets a program pick `ssl:s` by defining a weak `__nx_ssl_service_type` global, and
//! reading that from Rust would mean declaring a weak *undefined* symbol, which needs an unstable
//! compiler feature. So this connects to `ssl` and nothing else, and the two commands that exist
//! only on the system variant report the service as uninitialized, exactly as they do for any
//! program on the default one.
//!
//! Nothing is lost for the programs this workspace targets: `ssl:s` needs permissions homebrew
//! does not have.
//!
//! ## What it does not hold
//!
//! Nothing sends a request on the caller's behalf beyond what the C API describes. Removing a PKI
//! searches three places for an id, because the id does not say which kind it is, but only while
//! the service keeps answering "not here": any other failure is reported rather than searched
//! past.
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

#[cfg(feature = "ffi")]
pub mod ffi;
