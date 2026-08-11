//! # nx-rt-module
//!
//! Dynamically-loadable-module entry crate for the Nintendo Switch runtime
//! crate family.
//!
//! `nx-rt-module` is the runtime for one output kind: a relocatable `NRO`
//! module that an already-running process loads at runtime through the `ro`
//! service. Unlike an application or a sysmodule, a module is **not a
//! process**: it has no `_start`, opens no environment, and is launched by
//! neither the homebrew loader nor the process manager. The host process maps
//! it, the `ro` service applies its relocations after checking it against a
//! registration blob (`NRR`), and the host then runs the module's
//! constructors.
//!
//! ## Output-kind row
//!
//! | App type / kind | Executable | Launched by | Applet type | Applet type sourced |
//! |-----------------|-----------|-------------|-------------|---------------------|
//! | Dynamically loadable module | NRO + NRR | `ro` dynamic load | inherited from host | n/a |
//!
//! See [`nx_rt_core`] for the full App-Type / Output-Kind matrix covering
//! every Switch executable kind.
//!
//! ## Module versus process
//!
//! Every other entry crate in the family backs a *process*: it owns a
//! `_start`, brings up the heap, parses an environment, opens a Service
//! Manager session, and registers an Application Manager identity. A module
//! owns none of that. It is code grafted into a host process that has already
//! done all of it, so a module **inherits** its runtime rather than
//! **initializing** one:
//!
//! - **No entry point**: a module has no `_start`. The `ro` service relocates
//!   it; there is no kind-specific startup ABI for this crate to implement.
//! - **No environment**: the heap, the Horizon OS version, the main-thread
//!   TLS, and the Service Manager session all belong to the host process. A
//!   module reuses them as they are and never re-runs [`nx_rt_core`]'s
//!   process bring-up.
//! - **No applet identity**: the Application Manager identity is whatever the
//!   host process registered. A module never selects an applet type and never
//!   runs an applet handshake.
//!
//! What a module *does* own is its own lifecycle inside the host: running its
//! static constructors once relocation is complete, and its destructors before
//! it is unloaded. That glue is [`init`].
//!
//! ## Export visibility
//!
//! A loadable module is reached through its dynamic symbol table: the host
//! resolves the module's exports against the `NRR` it was registered with.
//! Which symbols land in `.dynsym` is governed by symbol visibility at link
//! time, not by this crate's Rust surface. Owning that export-visibility
//! surface belongs with the module's final link and is handled there.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

pub mod init;
