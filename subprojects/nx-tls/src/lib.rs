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
//! symbols needing something the runtime holds — a service-manager session, the firmware version —
//! get stranded in the runtime, away from the surface they belong to. `nx-net` is split that way
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
//! ## What it does not do yet
//!
//! The `ssl` C surface is large — contexts, connections, handshakes, certificate handling — and
//! only the socket hand-offs are ported. Those are the ones a socket program reaches first: they
//! take a descriptor from the process's table and give it to a TLS connection, which is the one
//! operation that needs a layer knowing both the descriptor table and the service. The rest is a
//! port of [`nx_service_ssl`]'s surface with no such crossing in it, and it lands here as it is
//! written.
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

#[cfg(feature = "ffi")]
pub mod ffi;
