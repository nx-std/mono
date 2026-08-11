//! # nx-rt-nso
//!
//! NSO-process entry crate for the Nintendo Switch runtime crate family.
//!
//! `nx-rt-nso` is the runtime for one output kind: the `NSO` process
//! launched by the process manager (`pm`): installed applications, every
//! system-applet kind, and background sysmodules. It stacks the NSO-specific
//! startup on top of the kind-agnostic [`nx_rt_core`]: the `pm`-handoff
//! bring-up (no homebrew-loader configuration block), the SVC-backed heap
//! path, the `__argdata__` command-line (`argv`) reader, and the
//! build-time-selected Application Manager (applet) handshake.
//!
//! ## Output-kind rows
//!
//! | App type / kind | Executable | Launched by | Applet type | Applet type sourced |
//! |-----------------|-----------|-------------|-------------|---------------------|
//! | Regular application | NSO | `pm` | `Application` | **Build time** |
//! | System applet (qlaunch) | NSO | `pm` | `SystemApplet` | **Build time** |
//! | Library applet (system) | NSO | `pm` | `LibraryApplet` | **Build time** |
//! | Overlay applet | NSO | `pm` | `OverlayApplet` | **Build time** |
//! | System application | NSO | `pm` | `SystemApplication` | **Build time** |
//! | Background sysmodule | NSO | `pm` | `None` | **Build time** |
//!
//! Unlike a homebrew NRO, which receives its applet type at runtime from the
//! homebrew loader's configuration block, every NSO selects its applet type
//! at build time. All six Application Manager identities share the single NSO
//! startup ABI; the build picks one, producing an applet-type *value* that
//! flows into the applet handshake. A `None` background sysmodule skips the
//! Application Manager entirely. See [`nx_rt_core`] for the full App-Type /
//! Output-Kind matrix covering every Switch executable kind.
//!
//! ## Background sysmodule (`None`) profile
//!
//! Selecting `nso_applet_type=none` builds the runtime for a background
//! sysmodule: a `pm`-launched NSO that exists to provide a service and has no
//! Application Manager identity. Its startup profile is deliberately minimal:
//!
//! - **Service set**: only the Service Manager (`sm`) is brought up. The
//!   Application Manager handshake is skipped, so no `appletOE` / `appletAE`
//!   proxy session is opened and no AM handle is held. A sysmodule that needs
//!   a further service opens it explicitly; nothing else starts on its behalf.
//! - **Applet identity**: the `__nx_applet_type` global reports `None`. That
//!   value *is* the skip signal: the applet-init entry point returns before
//!   contacting the Application Manager, and the libnx applet runtime likewise
//!   treats `None` as "do not initialize".
//!
//! Every other `nso_applet_type` selection registers one of the five Application
//! Manager identities and runs its per-role handshake; see [`applet`] for that
//! mapping.
//!
//! ## Startup capability fragment
//!
//! An NSO process declares the supervisor calls it may invoke and the system
//! services it may reach in its NPDM. Those permissions are the union of what
//! the application needs and what its runtime startup needs. [`caps`] owns the
//! *runtime* half as inspectable data (a [`caps::CapabilityFragment`] keyed by
//! applet identity) so a build tool can merge it with the application-declared
//! capabilities instead of an NPDM being hand-written. The fragment varies with
//! the build-time applet type: a background sysmodule needs no Application
//! Manager service access; a foreground applet additionally needs the
//! synchronization calls its focus-wait handshake invokes.
//!
//! # Cargo features
//!
//! - `ffi`: gates the [`ffi`] module: the `__nx_rt_nso__libnx_*` C-FFI symbols that
//!   redirect the NSO-specific `libnx` runtime entry points (`envSetup`,
//!   `argvSetup`, `appletInitialize`, `__nx_applet_type`). Without it no
//!   override symbols are emitted and the linker fragments have nothing to
//!   bind. The kind-agnostic runtime symbols are owned by [`nx_rt_core`]'s
//!   FFI surface.
//! - `rt-link`: emits this crate's `pm`-launch `.crt0` (the NSO process
//!   `_start`) for the opt-in `rustc`-driven link pipeline. It is off on the
//!   default GCC pipeline, where `_start` is supplied by libnx's
//!   `switch_crt0.s`; enabling it there would collide with that `_start`.
//!
//! # Build-time cfg
//!
//! - `nso_applet_type`: the build-time Application Manager identity, one of
//!   `application`, `library-applet`, `none`, `overlay-applet`,
//!   `system-applet` or `system-application`. The `nso_applet_type` Meson
//!   option sets it, and the workspace `.cargo/config.toml` supplies the
//!   `application` default a bare `cargo` invocation builds with. Any other
//!   value, or none at all, is a `compile_error!`. It is a cfg value rather
//!   than a set of Cargo features because the six identities are mutually
//!   exclusive, and Cargo features are additive: `--all-features` would turn
//!   all six on at once.

#![no_std]

extern crate nx_alloc as _; // provides #[global_allocator]
extern crate nx_panic_handler as _; // provides #[panic_handler]

// `pm` process-launch `.crt0` startup section for the `rustc`-link pipeline.
// Gated behind `rt-link` so the `_start` it defines is emitted only when
// `rustc` drives the final link; on the GCC pipeline `_start` comes from
// libnx's `switch_crt0.s`, and an unconditional `.crt0` would collide with it.
#[cfg(feature = "rt-link")]
core::arch::global_asm!(include_str!("crt0.s"));

// Build-time Application Manager identity selection.
//
// `applet::APPLET_TYPE` has one arm per identity, each naming its own
// `nso_applet_type` value. A cfg that is unset, or that carries a value outside
// the six, matches no arm; this guard reports that as the misconfiguration it
// is, instead of leaving a cryptic "`APPLET_TYPE` is not defined" behind or
// letting the process register as an identity nobody asked for.
#[cfg(not(any(
    nso_applet_type = "application",
    nso_applet_type = "library-applet",
    nso_applet_type = "none",
    nso_applet_type = "overlay-applet",
    nso_applet_type = "system-applet",
    nso_applet_type = "system-application",
)))]
compile_error!(
    "nx-rt-nso: the `nso_applet_type` cfg is unset or names an unknown applet \
     type; set the `nso_applet_type` Meson option (the workspace \
     `.cargo/config.toml` carries the `application` default for bare cargo \
     invocations)"
);

#[cfg(feature = "ffi")]
pub mod ffi;

pub mod applet;
pub mod argv;
pub mod caps;
pub mod env;
pub mod init;
