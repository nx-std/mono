//! Fatal error service (`fatal:u`) implementation.
//!
//! Provides an interface for throwing system fatal errors with configurable
//! error reporting and screen display policies.
//!
//! ## The service
//!
//! `fatal:u` is how a process reports an unrecoverable failure to the system.
//! The caller hands it a result code and a [`FatalPolicy`] saying what the
//! system should do with it: write an error report, show the fatal error
//! screen, or both. Whether the call returns is the policy's decision, not the
//! caller's - [`FatalPolicy::ErrorReport`] records the failure and comes back,
//! while the screen-showing policies do not.
//!
//! [`throw_fatal_with_context`](FatalService::throw_fatal_with_context) adds a
//! [`FatalCpuContext`]: the register state, stack trace and exception type at
//! the point of failure, which the system records alongside the report. The
//! context is optional because the other two entry points describe failures a
//! process detected itself, where its own register state says nothing useful.
//!
//! ## Architecture
//!
//! One connected session, [`FatalService`], obtained from [`connect_cmif`].
//! The three throw commands differ only in how much they describe: a bare
//! result code, a result code plus a policy, or both plus a CPU context.
//!
//! ## Divergence from libnx
//!
//! libnx's `fatal.c` performs a hosversion check to downgrade
//! [`FatalPolicy::ErrorScreen`] to [`FatalPolicy::ErrorReportAndErrorScreen`]
//! on firmware versions before 3.0.0, calls `detectDebugger()` +
//! `svcBreak` before sending the IPC request, and calls `svcExitProcess()`
//! after certain policies. This crate follows the convention of the other
//! `nx-service-*` crates: it exposes the raw IPC commands and leaves
//! hosversion gating, debugger detection, and process exit to the caller.
//!
//! Per IC-4, this crate is hosversion-unaware — callers choose the
//! appropriate policy based on the target firmware version.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{
    BorrowedSessionHandle,
    Session,
};

mod cmif;
mod proto;
pub mod types;

pub use self::{
    cmif::ThrowFatalError,
    proto::SERVICE_NAME,
    types::{
        FatalAarch64Context,
        FatalCpuContext,
        FatalPolicy,
    },
};

/// Fatal error (`fatal:u`) session wrapper.
#[repr(transparent)]
pub struct FatalService(Session);

impl FatalService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods for `fatal:u`.
impl FatalService {
    /// Throws a fatal error with the given policy (no CPU context).
    #[inline]
    pub fn throw_fatal_with_policy(
        &self,
        result_code: u32,
        policy: FatalPolicy,
    ) -> Result<(), ThrowFatalError> {
        cmif::throw_fatal_with_policy(self.0.handle(), result_code, policy)
    }

    /// Throws a fatal error with the given policy and CPU context.
    #[inline]
    pub fn throw_fatal_with_context(
        &self,
        result_code: u32,
        policy: FatalPolicy,
        ctx: &FatalCpuContext,
    ) -> Result<(), ThrowFatalError> {
        cmif::throw_fatal_with_context(self.0.handle(), result_code, policy, ctx)
    }
}

/// Connects to the `fatal:u` service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<FatalService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(FatalService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get fatal:u service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
