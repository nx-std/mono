//! Process manager (`pm`) service implementation.
//!
//! Provides access to the process manager services for launching programs,
//! managing processes, querying boot mode, and inspecting process state.
//!
//! ## Service Variants
//!
//! Four service endpoints are available:
//!
//! - **`pm:bm`** — Boot mode service. Connected via [`connect_bm_cmif`].
//! - **`pm:dmnt`** — Debug/monitor service. Connected via [`connect_dmnt_cmif`].
//! - **`pm:info`** — Process info service. Connected via [`connect_info_cmif`].
//! - **`pm:shell`** — Shell service. Connected via [`connect_shell_cmif`].
//!
//! ## Hosversion variants
//!
//! `pm:dmnt` and `pm:shell` commands were renumbered at `[5.0.0]`. Methods
//! that have different command IDs across versions are exposed as paired
//! `_legacy` (pre-5.0.0) and non-suffixed (5.0.0+) variants. The caller
//! selects based on the system version.
//!
//! Commands that only exist on newer firmware (e.g., `ClearHook` on 6.0.0+,
//! `BoostApplicationThreadResourceLimit` on 7.0.0+) are exposed
//! unconditionally.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

mod cmif;
mod dispatch;
mod pm_bm;
mod pm_dmnt;
mod pm_info;
mod pm_shell;
mod types;

pub use self::{
    pm_bm::{
        BootMode,
        ConnectBmCmifError,
        GetBootModeError,
        PmBmService,
        UnknownBootMode,
        connect_bm_cmif,
    },
    pm_dmnt::{
        ConnectDmntCmifError,
        PmDmntService,
        connect_dmnt_cmif,
    },
    pm_info::{
        ConnectInfoCmifError,
        PmInfoService,
        ResourceLimitValues,
        connect_info_cmif,
    },
    pm_shell::{
        ConnectShellCmifError,
        GetProcessEventInfoError,
        LaunchFlag,
        LaunchFlagOld,
        NcmProgramLocation,
        PmShellService,
        ProcessEvent,
        ProcessEventInfo,
        UnknownProcessEvent,
        connect_shell_cmif,
    },
    types::{
        ProcessId,
        ProgramId,
    },
};
