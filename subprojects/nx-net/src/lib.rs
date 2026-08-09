//! # nx-net
//!
//! DNS resolution for Nintendo Switch / Horizon OS homebrew — a Rust
//! replacement for the C resolver (`getaddrinfo` and the legacy
//! `netdb` family). The crate covers **name resolution only**; socket
//! transport APIs (`socket`, `connect`, `send`, `recv`, TCP/UDP) are out of
//! scope.
//!
//! ## Backend
//!
//! Horizon OS performs DNS resolution in a system process and exposes it
//! through the `sfdnsres` IPC service. `nx-net` does not implement that IPC
//! itself: the [`nx_service_sfdnsres`] crate is the typed CMIF client and is
//! the sole transport this crate builds on. `nx-net` owns the layers
//! `nx-service-sfdnsres` deliberately leaves out — the wire-format codec, the
//! idiomatic Rust resolver API, the C result-allocation contract, and the
//! C-ABI FFI.
//!
//! ## Three-layer architecture
//!
//! ```text
//!   C callers
//!         |  __nx_net__* symbols           layer 3: C-ABI FFI
//!   +-----v-------------------------+
//!   | ffi   hard shell: validate    |
//!   |       every C input, build /  |
//!   |       free result blocks;     |
//!   |       abi  repr(C) BSD structs|
//!   +-----+-------------------------+
//!         |  validated Rust types          layer 2: musl-shaped Rust API
//!   +-----v-------------------------+
//!   | resolve   std-like facade;    |
//!   |  resolver getaddrinfo,        |
//!   |           gethostbyname*,     |
//!   |           getnameinfo, ...    |
//!   +-----+-------------------------+
//!         |                                layer 1: soft core
//!   +-----v-------------------------+
//!   | resolve::{hostname, service,  |
//!   |   family, hints}  newtypes    |
//!   +-------------------------------+
//!         |
//!   nx-service-sfdnsres  (sfdnsres CMIF transport + wire codec)
//! ```
//!
//! 1. **Soft core** — idiomatic, no-std Rust: validated input newtypes. It
//!    carries no C-ABI item — the input enums hold their C numeric values as
//!    discriminants. Once an input has crossed into a core type it is trusted;
//!    the core performs no defensive re-validation. The serialized `sfdnsres`
//!    wire-format codec and the owned decoded result types live in
//!    `nx-service-sfdnsres`; the resolver consumes and re-exports them.
//! 2. **musl-shaped Rust API** — a resolver module whose function set and
//!    naming mirror musl's `src/network` (`getaddrinfo`, `freeaddrinfo`,
//!    `gai_strerror`, `getnameinfo`, `gethostbyname`, `gethostbyname2`,
//!    `gethostbyaddr`, `freehostent`, `hstrerror`, `herror`) but takes and
//!    returns Rust types instead of raw C pointers.
//! 3. **C-ABI FFI** — `__nx_net__*` symbols and the entire C-ABI (the
//!    `repr(C)` BSD structs and the `AF_*`/`EAI_*`/… integer constants),
//!    gated behind `feature = "ffi"`. The exports form the hard shell: they
//!    validate every C input and convert it into core types before calling
//!    the resolver. A `net_override.ld` linker script redirects the C
//!    resolver symbols onto these at link time.
//!
//! ## no-std
//!
//! The crate is `#![no_std]` and targets `aarch64-nintendo-switch-freestanding`
//! with `panic = "abort"`. It uses `alloc` heap types (`Box`/`Vec`/`String`)
//! for the decoded result types and the C result blocks; the umbrella
//! `nx-std` crate owns the single `#[global_allocator]`.
//!
//! The `ffi` feature additionally needs `#![feature(thread_local)]`: writing
//! the C resolver's thread-local `h_errno` requires a `#[thread_local]`
//! reference to the variable the linked C runtime defines.
#![no_std]
#![cfg_attr(feature = "ffi", feature(thread_local))]

extern crate alloc; // Box/Vec/String + getaddrinfo result block
extern crate nx_alloc; // umbrella owns #[global_allocator]
extern crate nx_panic_handler as _; // provides #[panic_handler]

pub mod resolve;

#[cfg(feature = "ffi")]
pub mod ffi;

pub use resolve::{
    LookupHost,
    lookup_host,
    lookup_ip,
    target::{
        ToSocketAddrs,
        ToSocketAddrsError,
    },
};
