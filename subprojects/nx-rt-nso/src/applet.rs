//! # Application Manager (applet) bring-up (NSO)
//!
//! Exposes the single applet-init entry point for an NSO process. The applet
//! handshake itself — the per-role libnx-faithful bring-up and the runtime
//! singleton — is kind-agnostic and lives in [`nx_rt_core::services::applet`];
//! this module only sources the [`AppletType`] value the handshake consumes.
//!
//! Unlike a homebrew NRO — which receives its applet type at runtime from the
//! homebrew loader's configuration block — an NSO has no loader block to read.
//! Its Application Manager identity is fixed when the process image is built,
//! so the applet type flows in as the [`APPLET_TYPE`] build-time value rather
//! than a parsed-at-runtime one.
//!
//! ## Applet-type coverage
//!
//! Each `nso_applet_type` Meson value selects exactly one `applet-*` Cargo
//! feature, which fixes [`APPLET_TYPE`]. [`applet_init`] hands that value to
//! [`nx_rt_core::services::applet::init`], which dispatches to the matching
//! per-role `open_*` helper and brings up the Application Manager proxy
//! command shown below:
//!
//! | `nso_applet_type`    | [`APPLET_TYPE`]              | per-role helper            | AM proxy command       |
//! |----------------------|-----------------------------|----------------------------|------------------------|
//! | `application`        | [`AppletType::Application`]  | `open_application`         | `appletOE` cmd 0       |
//! | `system-applet`      | [`AppletType::SystemApplet`]| `open_system_applet`       | `appletAE` cmd 100     |
//! | `library-applet`     | [`AppletType::LibraryApplet`]| `open_library_applet`     | `appletAE` cmd 200·201 |
//! | `overlay-applet`     | [`AppletType::OverlayApplet`]| `open_overlay_applet`     | `appletAE` cmd 300     |
//! | `system-application` | [`AppletType::SystemApplication`]| `open_system_application` | `appletAE` cmd 350 |
//! | `none`               | [`AppletType::None`]        | — (handshake skipped)      | none                   |
//!
//! The `none` background-sysmodule profile contacts no Application Manager:
//! `init` returns early before opening any proxy, so the process burns no AM
//! handle.

pub use applet::ConnectError;
use nx_rt_core::services::applet;
use nx_service_applet::AppletType;
use nx_svc::process::Handle as ProcessHandle;

/// The Application Manager identity this NSO process registers as.
///
/// Every NSO declares exactly one applet type for the lifetime of the build.
/// Unlike a homebrew NRO — which the loader hands an applet type at runtime —
/// an NSO has no loader block, so the identity is fixed in the process image.
///
/// The value is selected at build time by the active `applet-*` Cargo feature,
/// driven by the `nso_applet_type` Meson option. Exactly one
/// `applet-*` feature must be enabled; the crate root ([`crate`]) enforces this
/// with a `compile_error!` guard.
#[cfg(feature = "applet-application")]
pub const APPLET_TYPE: AppletType = AppletType::Application;
#[cfg(feature = "applet-library-applet")]
pub const APPLET_TYPE: AppletType = AppletType::LibraryApplet;
#[cfg(feature = "applet-none")]
pub const APPLET_TYPE: AppletType = AppletType::None;
#[cfg(feature = "applet-overlay-applet")]
pub const APPLET_TYPE: AppletType = AppletType::OverlayApplet;
#[cfg(feature = "applet-system-applet")]
pub const APPLET_TYPE: AppletType = AppletType::SystemApplet;
#[cfg(feature = "applet-system-application")]
pub const APPLET_TYPE: AppletType = AppletType::SystemApplication;

/// Brings up the Application Manager handshake for this NSO's build-time
/// applet identity ([`APPLET_TYPE`]).
///
/// Each of the five applet roles runs its libnx-faithful per-role handshake;
/// [`AppletType::None`] — a background sysmodule — skips the Application
/// Manager entirely. `process_handle` is this process's own handle, which the
/// Application Manager associates with the applet.
///
/// # Panics
///
/// Panics if the Service Manager is not yet initialized.
pub fn applet_init(process_handle: ProcessHandle) -> Result<(), ConnectError> {
    applet::init(APPLET_TYPE, process_handle)
}
