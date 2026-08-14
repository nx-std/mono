//! # nx-rt-core
//!
//! Launch-path-agnostic foundation for the Nintendo Switch runtime crate
//! family.
//!
//! `nx-rt-core` holds the runtime machinery that every Switch executable
//! shares, regardless of how it is launched or linked: the parsed
//! environment-state container and its read accessors, heap initialization,
//! Horizon OS version detection, syscall hints, main-thread TLS setup, the
//! panic glue, and the Service Manager (`sm`) bootstrap.
//!
//! The command line is deliberately absent. Each entry crate reads it from the
//! source its own launch path provides and installs it in `nx-sys-args`, which
//! holds it the way `std::sys::args` does: below every caller, rather than in
//! the last crate of the graph where nothing else could reach it.
//!
//! It deliberately contains **no** process entry point, no
//! loader-configuration parser, and no Application Manager (applet) logic.
//! Those concerns belong to the per-launch-path entry crates that depend on
//! this one.
//!
//! ## The runtime crate family
//!
//! A Switch binary is entered exactly one way: handed over by the homebrew
//! loader, launched by `pm`, launched by the kernel, or loaded into a running
//! host process by `ro`. Each of those *launch paths* gets its own entry crate
//! stacked on top of `nx-rt-core`; a final binary depends on exactly one entry
//! crate, and that dependency *is* its launch path. The launch path cannot be a
//! Cargo feature, because features are additive and unify across a dependency
//! graph: they cannot model mutual exclusivity.
//!
//! The *applet type*, the Application Manager identity a process registers as
//! (`Application`, `SystemApplet`, `LibraryApplet`, `OverlayApplet`,
//! `SystemApplication`, or `None` for a background sysmodule), is **not** a
//! separate entry crate. All Application Manager identities share one process
//! startup ABI; only which AM proxy to open (or whether to skip AM entirely)
//! differs. The applet type is therefore a runtime-profile sub-axis: homebrew
//! NROs source it at runtime from the loader configuration, while NSO and KIP
//! processes select it at build time. It always flows as a value into the
//! applet-init entry point: never as a link-time global.
//!
//! ## App-Type / Launch-Path matrix
//!
//! This table enumerates every Switch executable the `nx-rt-*` family serves,
//! including each applet role, and maps each to the entry crate its launch
//! path selects, and to its runtime profile.
//!
//! | App type | Executable | Launched by | Entry crate | Applet type | Applet type sourced | AM behavior |
//! |----------|-----------|-------------|-------------|-------------|---------------------|-------------|
//! | Homebrew application | NRO | hbloader (hbmenu) | `nx-rt-hbapp` | `Application` (or `LibraryApplet` on album-launch; `SystemApplication`→`Application` via `ApplicationOverride`) | **Runtime**: hbl `EntryType_AppletType` config | `appletOE` cmd 0 / `appletAE` cmd 200·201 |
//! | Homebrew library applet | NRO | hbloader / album | `nx-rt-hbapp` | `LibraryApplet` | **Runtime**: hbl config | `appletAE` cmd 200·201 |
//! | Regular application | NSO | `pm` | `nx-rt-nso` | `Application` | **Build time** | `appletOE` cmd 0 |
//! | System applet (qlaunch) | NSO | `pm` | `nx-rt-nso` | `SystemApplet` | **Build time** | `appletAE` cmd 100 |
//! | Library applet (system) | NSO | `pm` | `nx-rt-nso` | `LibraryApplet` | **Build time** | `appletAE` cmd 200·201 |
//! | Overlay applet | NSO | `pm` | `nx-rt-nso` | `OverlayApplet` | **Build time** | `appletAE` cmd 300 |
//! | System application | NSO | `pm` | `nx-rt-nso` | `SystemApplication` | **Build time** | `appletAE` cmd 350 |
//! | Background sysmodule | NSO | `pm` | `nx-rt-nso` | `None` | **Build time** | skip AM entirely |
//! | Dynamically loadable module | NRO + NRR | `ro` dynamic load | `nx-rt-module` | inherited from host process | n/a | n/a (no own `_start`) |
//! | Boot-time sysmodule | KIP | kernel | `nx-rt-kip` | `None` | **Build time** | skip AM entirely |
//!
//! The entry-crate axis is the **launch path**: not the output format, not the
//! app role and not the applet type. The format cannot be the axis, as two rows
//! above show: both are an `NRO`, but the loader hands one over as a process of
//! its own while `ro` loads the other into a host process already running. All
//! six AM identities share the `pm`-launch startup ABI, so the applet type is a
//! runtime-profile sub-axis owned by the entry crates, never a fifth, sixth, …
//! entry crate.
#![no_std]

extern crate alloc;
extern crate nx_panic_handler as _; // provides #[panic_handler]

pub mod caps;
pub mod env;
#[cfg(feature = "ffi")]
pub mod error;
#[cfg(feature = "ffi")]
pub mod ffi;
pub mod init;
pub mod services;
