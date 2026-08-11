//! # nx-rt-kip
//!
//! Boot-time-KIP entry crate for the Nintendo Switch runtime crate family.
//!
//! `nx-rt-kip` is the runtime for one output kind: a `KIP` (Kernel Initial
//! Process), a boot-time sysmodule the kernel itself launches while the system
//! is coming up, before the process manager (`pm`) and the homebrew loader
//! exist. It stacks the KIP-specific startup on top of the kind-agnostic
//! [`nx_rt_core`]: a kernel-launch entry ABI, the SVC-backed heap path, and a
//! `None` applet identity that never contacts the Application Manager.
//!
//! ## Output-kind row
//!
//! | App type / kind | Executable | Launched by | Applet type | Applet type sourced |
//! |-----------------|-----------|-------------|-------------|---------------------|
//! | Boot-time sysmodule | KIP | kernel | `None` | Build time (fixed) |
//!
//! See [`nx_rt_core`] for the full App-Type / Output-Kind matrix covering
//! every Switch executable kind.
//!
//! ## KIP-launch runtime profile
//!
//! A KIP is launched directly by the kernel: there is no process manager to
//! hand it a launch parameter and no homebrew loader to hand it a configuration
//! block. Its runtime profile is therefore fixed by the output kind rather than
//! selected per build, and it is the most minimal profile in the family:
//!
//! - **Kernel-launch entry ABI**: the kernel maps the KIP's segments from the
//!   `KIP` image and jumps to its entry point. There is no `pm` handoff and no
//!   loader environment block; the KIP brings its environment up from nothing.
//!   The entry-point `.crt0` that implements this ABI is owned by this crate
//!   and emitted on the opt-in `rustc`-link pipeline behind the `rt-link`
//!   feature (see [Cargo features](#cargo-features) below).
//! - **No command line**: a kernel-launched process receives no `argv`. The
//!   KIP startup runs no command-line scan, so it never probes a command-line
//!   memory region.
//! - **Fixed `None` applet identity**: a boot-time sysmodule exists to provide
//!   a service and has no Application Manager identity. The applet type is
//!   fixed `None`; the Application Manager handshake is skipped entirely, so no
//!   `appletOE` / `appletAE` proxy session is ever opened. Unlike an NSO, whose
//!   applet identity is one of six build-time selections, a KIP has no choice
//!   to make.
//!
//! What the KIP startup *does* own is the kind-agnostic bring-up [`nx_rt_core`]
//! provides (the SVC-backed heap, the Horizon OS version, the main-thread TLS,
//! and the Service Manager (`sm`) session), driven over the kernel-launch ABI.
//!
//! ## Startup capability fragment
//!
//! A KIP declares the supervisor calls it may invoke directly inside its `KIP`
//! header's kernel-capability descriptors: there is no separate NPDM. Those
//! permissions are the union of what the sysmodule itself needs and what its
//! runtime startup needs. [`caps`] owns the *runtime* half as inspectable data
//! (a [`caps::CapabilityFragment`]) so a build tool can merge it with the
//! sysmodule-declared capabilities instead of a KIP header being hand-written.
//!
//! # Cargo features
//!
//! - `rt-link`: emits this crate's kernel-launch `.crt0` (the KIP process
//!   `_start`) for the opt-in `rustc`-driven link pipeline. It is off on the
//!   default GCC pipeline, where `_start` is supplied by libnx's
//!   `switch_crt0.s`; enabling it there would collide with that `_start`.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

// Kernel-launch `.crt0` startup section for the `rustc`-link pipeline.
// Gated behind `rt-link` so the `_start` it defines is emitted only when
// `rustc` drives the final link; on the GCC pipeline `_start` comes from
// libnx's `switch_crt0.s`, and an unconditional `.crt0` would collide with it.
#[cfg(feature = "rt-link")]
core::arch::global_asm!(include_str!("crt0.s"));

pub mod caps;
