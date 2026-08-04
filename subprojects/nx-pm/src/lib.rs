//! # nx-pm
//!
//! Idiomatic Rust API for Horizon OS process-manager services, built on
//! [`nx_service_pm`]'s CMIF transport.
//!
//! ## What `pm` is
//!
//! `pm` (Process Manager) is the Horizon OS sysmodule that owns the lifecycle
//! of every other userland process on the console: it asks `ldr:pm` to load
//! NSO/KIP binaries, brings up resource-limit groups, brokers process-creation
//! hooks for the debug monitor, and reports start / exit / crash / debug
//! events. On retail consoles `pm` is the Nintendo binary; on Atmosphère-NX
//! it is re-implemented by `stratosphere/pm` (`pm.kip`) with two extension
//! commands (`AtmosphereGetProcessInfo`, `AtmosphereHasLaunchedBootProgram`,
//! ...) on top of the stock interface.
//!
//! The sysmodule exposes four named service endpoints. Each is a separate
//! service registration with its own session pool (session counts reserved
//! by AMS in parentheses: shell 8, dmnt 16, bm 8, info 16 — 48 total):
//!
//! | Service    | Purpose                                                       | Typical caller            |
//! |------------|---------------------------------------------------------------|---------------------------|
//! | `pm:shell` | Launch / terminate programs, drain process events, boost RL.  | `am`, `ns`, recovery flow |
//! | `pm:dmnt`  | Debug hooks, JIT-debug PID list, start suspended processes.   | `creport`, `dmnt`, gdb    |
//! | `pm:bm`    | Read / set boot mode (Normal / Maintenance / SafeMode).       | `set:sys`, recovery flow  |
//! | `pm:info`  | Process → program ID, applet resource-limit values.           | Most homebrew / sysmods   |
//!
//! For homebrew, `pm:info` is the only endpoint that is freely accessible;
//! `pm:shell` and `pm:dmnt` require sysmodule-tier permissions and are gated
//! by `sm:` ACLs on retail firmware.
//!
//! ## Surface in this crate
//!
//! One service-object type per endpoint, plus an RAII wrapper for the process
//! event handle:
//!
//! - [`BootModeService`] — `pm:bm`
//! - [`DebugMonitorService`] — `pm:dmnt`
//! - [`ProcessInfoService`] — `pm:info`
//! - [`ShellService`] — `pm:shell`
//! - [`ProcessEvent`] — owned kernel event returned by `pm:shell`
//!   `GetProcessEventHandle` and `pm:dmnt` `HookToCreateProcess`. Auto-clear
//!   is always enabled (matches `pmshellProcessEvent` in libnx).
//!
//! Each service is obtained by passing an [`SmService`] handle to the
//! corresponding `connect_*` free function; the resulting object owns the
//! session and disconnects on drop.
//!
//! ## Layering
//!
//! ```text
//!   nx-pm            BootMode / DebugMonitor / ProcessInfo / Shell + RAII events
//!     |
//!   nx-service-pm    raw CMIF wrappers (PmBmService, PmDmntService, ...)
//!     |
//!     libnx pm IPC   pm:bm / pm:dmnt / pm:info / pm:shell
//! ```
//!
//! ## Firmware variants
//!
//! Horizon renumbered `pm:shell` and `pm:dmnt` command IDs at `[5.0.0]` and
//! reshuffled two wire formats. Libnx and Atmosphère ship paired interfaces
//! (`IShellInterface` / `IDeprecatedShellInterface`, same for dmnt) and pick
//! at session-accept time based on the running HOS version.
//!
//! This crate exposes the renumbered commands as paired methods: the bare
//! name targets `[5.0.0+]`, the `_legacy` suffix targets pre-5.0.0. The
//! caller picks based on the running firmware (e.g. via
//! `nx_rt::env::hos_version`). Commands that only exist on one side of the
//! split are exposed unconditionally and will surface a kernel error on the
//! wrong firmware (e.g. `BoostApplicationThreadResourceLimit` is `[7.0.0+]`,
//! `CleanupProcess` / `ClearExceptionOccurred` were dropped at `[5.0.0]`).
//!
//! Wire-level shapes that change with firmware and have explicit handling
//! in this crate:
//!
//! - [`LaunchFlags`] (`[5.0.0+]`) vs. [`LaunchFlagsLegacy`] — different bit
//!   layout for the `SignalOn*` / `StartSuspended` / `DisableAslr` flags.
//! - [`ProcessEventKind`] — variant values renumbered at `[5.0.0]`; the type
//!   surfaces the modern numbering and the legacy mapping is performed by
//!   `nx-service-pm`.
//!
//! ## References
//!
//! - Switchbrew wiki: <https://switchbrew.org/wiki/PM_services>
//! - libnx: `src/nx/source/services/pm.c` and `include/switch/services/pm.h`
//! - Atmosphère: `stratosphere/pm/source` and
//!   `libraries/libstratosphere/include/stratosphere/pm`

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;

mod boot_mode;
mod debug_monitor;
mod event;
#[cfg(feature = "ffi")]
pub mod ffi;
mod process_info;
mod shell;

pub use self::{
    boot_mode::{
        BmGetBootModeError,
        BmSetMaintenanceBootError,
        BootMode,
        BootModeService,
    },
    debug_monitor::{
        DebugMonitorService,
        DmntClearHookError,
        DmntGetApplicationProcessIdError,
        DmntGetApplicationProcessIdLegacyError,
        DmntGetJitDebugProcessIdListError,
        DmntGetJitDebugProcessIdListLegacyError,
        DmntGetProcessIdError,
        DmntGetProcessIdLegacyError,
        DmntGetProgramIdError,
        DmntHookToCreateApplicationProcessError,
        DmntHookToCreateApplicationProcessLegacyError,
        DmntHookToCreateProcessError,
        DmntHookToCreateProcessLegacyError,
        DmntStartProcessError,
        DmntStartProcessLegacyError,
    },
    event::{
        HookId,
        ProcessEvent,
        ProcessHook,
        WaitError,
    },
    process_info::{
        InfoGetAppletCurrentResourceLimitValuesError,
        InfoGetAppletPeakResourceLimitValuesError,
        InfoGetProgramIdError,
        ProcessInfoService,
        ResourceLimitValues,
    },
    shell::{
        LaunchFlags,
        LaunchFlagsLegacy,
        ProcessEventInfo,
        ProcessEventKind,
        ProcessId,
        ProgramId,
        ProgramLocation,
        ShellBoostApplicationThreadResourceLimitError,
        ShellBoostSystemMemoryResourceLimitError,
        ShellBoostSystemMemoryResourceLimitLegacyError,
        ShellBoostSystemThreadResourceLimitError,
        ShellCleanupProcessError,
        ShellClearJitDebugOccurredError,
        ShellGetApplicationProcessIdForShellError,
        ShellGetApplicationProcessIdForShellLegacyError,
        ShellGetProcessEventHandleError,
        ShellGetProcessEventInfoError,
        ShellGetProcessIdError,
        ShellLaunchProgramError,
        ShellLaunchProgramLegacyError,
        ShellNotifyBootFinishedError,
        ShellNotifyBootFinishedLegacyError,
        ShellService,
        ShellTerminateProcessError,
        ShellTerminateProgramError,
    },
};

/// Opens a session to `pm:bm` (boot mode).
///
/// Asks `sm:` for a `pm:bm` handle over CMIF and wraps it in a
/// [`BootModeService`] that owns the session and closes it on drop. The
/// endpoint exposes `GetBootMode` / `SetMaintenanceBoot`; on retail firmware
/// access is restricted to system processes by the `sm:` ACL, so homebrew
/// running under `hbloader` will typically fail at the lookup step with
/// [`ConnectBmError`].
///
/// AMS reserves 8 sessions for `pm:bm`. The session pool is shared
/// process-wide; callers should keep the returned object alive only as long
/// as needed.
pub fn connect_bm(sm: &SmService) -> Result<BootModeService, ConnectBmError> {
    let inner = nx_service_pm::connect_bm_cmif(sm).map_err(ConnectBmError)?;
    Ok(BootModeService::new(inner))
}

/// Error returned by [`connect_bm`].
///
/// Wraps the underlying `sm:` lookup / session-setup failure. The most common
/// cause on retail firmware is missing permission to acquire `pm:bm`.
#[derive(Debug, thiserror::Error)]
#[error("failed to connect to pm:bm")]
pub struct ConnectBmError(#[source] pub nx_service_pm::ConnectBmCmifError);

/// Opens a session to `pm:dmnt` (debug monitor).
///
/// Asks `sm:` for a `pm:dmnt` handle over CMIF and wraps it in a
/// [`DebugMonitorService`] that owns the session and closes it on drop. The
/// endpoint backs debugger tooling (`creport`, `dmnt`, gdb stubs): hooking
/// process creation, enumerating JIT-debug PIDs, and starting suspended
/// processes. On retail firmware `pm:dmnt` is gated to debugger sysmodules
/// by the `sm:` ACL.
///
/// AMS reserves 16 sessions for `pm:dmnt`. Several commands renumbered at
/// `[5.0.0]`; see the crate-level "Firmware variants" section for how the
/// paired bindings on [`DebugMonitorService`] map to legacy / modern wire
/// formats.
pub fn connect_dmnt(sm: &SmService) -> Result<DebugMonitorService, ConnectDmntError> {
    let inner = nx_service_pm::connect_dmnt_cmif(sm).map_err(ConnectDmntError)?;
    Ok(DebugMonitorService::new(inner))
}

/// Error returned by [`connect_dmnt`].
///
/// Wraps the underlying `sm:` lookup / session-setup failure. On retail
/// firmware the most common cause is missing permission to acquire `pm:dmnt`.
#[derive(Debug, thiserror::Error)]
#[error("failed to connect to pm:dmnt")]
pub struct ConnectDmntError(#[source] pub nx_service_pm::ConnectDmntCmifError);

/// Opens a session to `pm:info` (process information).
///
/// Asks `sm:` for a `pm:info` handle over CMIF and wraps it in a
/// [`ProcessInfoService`] that owns the session and closes it on drop. This
/// is the only `pm` endpoint that is freely accessible to homebrew: it
/// exposes process-id ↔ program-id translation and applet resource-limit
/// values (see [`ProcessInfoService`]).
///
/// AMS reserves 16 sessions for `pm:info`.
pub fn connect_info(sm: &SmService) -> Result<ProcessInfoService, ConnectInfoError> {
    let inner = nx_service_pm::connect_info_cmif(sm).map_err(ConnectInfoError)?;
    Ok(ProcessInfoService::new(inner))
}

/// Error returned by [`connect_info`].
///
/// Wraps the underlying `sm:` lookup / session-setup failure. Because
/// `pm:info` is unrestricted, failures here typically indicate `sm:` itself
/// is unreachable rather than an ACL denial.
#[derive(Debug, thiserror::Error)]
#[error("failed to connect to pm:info")]
pub struct ConnectInfoError(#[source] pub nx_service_pm::ConnectInfoCmifError);

/// Opens a session to `pm:shell` (program lifecycle).
///
/// Asks `sm:` for a `pm:shell` handle over CMIF and wraps it in a
/// [`ShellService`] that owns the session and closes it on drop. The
/// endpoint drives program launch / termination, drains the global process
/// event stream, and brokers resource-limit boosts for `am` / `ns` / recovery
/// flows. On retail firmware `pm:shell` is gated to system processes by the
/// `sm:` ACL.
///
/// AMS reserves 8 sessions for `pm:shell`. Several commands renumbered at
/// `[5.0.0]`; see the crate-level "Firmware variants" section for how the
/// paired bindings on [`ShellService`] map to legacy / modern wire formats.
pub fn connect_shell(sm: &SmService) -> Result<ShellService, ConnectShellError> {
    let inner = nx_service_pm::connect_shell_cmif(sm).map_err(ConnectShellError)?;
    Ok(ShellService::new(inner))
}

/// Error returned by [`connect_shell`].
///
/// Wraps the underlying `sm:` lookup / session-setup failure. On retail
/// firmware the most common cause is missing permission to acquire
/// `pm:shell`.
#[derive(Debug, thiserror::Error)]
#[error("failed to connect to pm:shell")]
pub struct ConnectShellError(#[source] pub nx_service_pm::ConnectShellCmifError);
