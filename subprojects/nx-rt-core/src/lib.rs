//! # nx-rt-core
//!
//! Kind-agnostic foundation for the Nintendo Switch runtime crate family.
//!
//! `nx-rt-core` holds the runtime machinery that every Switch executable kind
//! shares, regardless of how it is launched or linked: the parsed
//! environment-state container and its read accessors, heap initialization,
//! Horizon OS version detection, syscall hints, main-thread TLS setup, the
//! command-line argument scanner, the panic glue, and the Service Manager
//! (`sm`) bootstrap.
//!
//! It deliberately contains **no** process entry point, no
//! loader-configuration parser, and no Application Manager (applet) logic.
//! Those concerns belong to the per-output-kind entry crates that depend on
//! this one.
//!
//! ## The runtime crate family
//!
//! A Switch binary is exactly one *output kind*: an `NRO`, an `NSO`, a
//! dynamically loadable module, or a boot-time `KIP`. Each output kind is a
//! distinct entry crate stacked on top of `nx-rt-core`; a final binary depends
//! on exactly one entry crate, and that dependency *is* its output kind. The
//! output kind cannot be a Cargo feature, because features are additive and
//! unify across a dependency graph: they cannot model mutual exclusivity.
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
//! ## App-Type / Output-Kind matrix
//!
//! This table enumerates every Switch executable kind the `nx-rt-*` family
//! serves, including each applet kind, and maps each to its entry crate and
//! runtime profile.
//!
//! | App type / kind | Executable | Launched by | Entry crate | Applet type | Applet type sourced | AM behavior |
//! |-----------------|-----------|-------------|-------------|-------------|---------------------|-------------|
//! | Homebrew application | NRO | hbloader (hbmenu) | `nx-rt-nro` | `Application` (or `LibraryApplet` on album-launch; `SystemApplication`→`Application` via `ApplicationOverride`) | **Runtime**: hbl `EntryType_AppletType` config | `appletOE` cmd 0 / `appletAE` cmd 200·201 |
//! | Homebrew library applet | NRO | hbloader / album | `nx-rt-nro` | `LibraryApplet` | **Runtime**: hbl config | `appletAE` cmd 200·201 |
//! | Regular application | NSO | `pm` | `nx-rt-nso` | `Application` | **Build time** | `appletOE` cmd 0 |
//! | System applet (qlaunch) | NSO | `pm` | `nx-rt-nso` | `SystemApplet` | **Build time** | `appletAE` cmd 100 |
//! | Library applet (system) | NSO | `pm` | `nx-rt-nso` | `LibraryApplet` | **Build time** | `appletAE` cmd 200·201 |
//! | Overlay applet | NSO | `pm` | `nx-rt-nso` | `OverlayApplet` | **Build time** | `appletAE` cmd 300 |
//! | System application | NSO | `pm` | `nx-rt-nso` | `SystemApplication` | **Build time** | `appletAE` cmd 350 |
//! | Background sysmodule | NSO | `pm` | `nx-rt-nso` | `None` | **Build time** | skip AM entirely |
//! | Dynamically loadable module | NRO + NRR | `ro` dynamic load | `nx-rt-module` | inherited from host process | n/a | n/a (no own `_start`) |
//! | Boot-time sysmodule | KIP | kernel | `nx-rt-kip` | `None` | **Build time** | skip AM entirely |
//!
//! The entry-crate axis is the output **format**: not the app role and not
//! the applet type. All six AM identities share the NSO startup ABI, so the
//! applet type is a runtime-profile sub-axis owned by the NRO and NSO entry
//! crates, never a fifth, sixth, … entry crate.
#![no_std]

extern crate alloc;
extern crate nx_panic_handler as _; // provides #[panic_handler]

pub mod argv;
pub mod caps;
pub mod env;
#[cfg(feature = "ffi")]
pub mod error;
#[cfg(feature = "ffi")]
pub mod ffi;
pub mod init;
pub mod services;
