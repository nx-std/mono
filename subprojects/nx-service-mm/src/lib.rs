//! Multimedia (`mm:u`) service implementation.
//!
//! Provides hardware module frequency management for multimedia
//! accelerators (RAM, NVENC, NVDEC, NVJPG) via the `mm:u` IPC
//! service.
//!
//! ## Hosversion variants
//!
//! The command surface changed at HOS 2.0.0. Pre-2.0.0 commands are
//! keyed by [`MmuModuleId`]; 2.0.0+ commands are keyed by a
//! server-assigned request ID. This crate exposes both sets of
//! methods (e.g. [`request_initialize`](MmService::request_initialize)
//! vs [`request_initialize_legacy`](MmService::request_initialize_legacy))
//! and leaves version selection to the caller.
//!
//! ## Divergence from libnx
//!
//! libnx's `mm.c` keeps a guarded global singleton (`g_mmuSrv`)
//! managed by `NX_GENERATE_SERVICE_GUARD` and dispatches to legacy or
//! modern commands at runtime via `hosversionBefore(2,0,0)`. This
//! crate follows the convention of the other `nx-service-*` crates:
//! connect once via [`connect_cmif`], reuse the [`MmService`] across
//! calls, and close the session explicitly with `Drop`.
//! Hosversion gating is the caller's responsibility.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::Session;
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{RequestFinalizeError, RequestGetError, RequestInitializeError, RequestSetAndWaitError},
    proto::SERVICE_NAME,
    types::{MmuModuleId, MmuRequest},
};

/// Multimedia service (`mm:u`) session wrapper.
#[repr(transparent)]
pub struct MmService(Session);

impl MmService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods (2.0.0+).
impl MmService {
    /// Initialises a multimedia request (2.0.0+).
    ///
    /// `unk` is ignored by official software.
    #[inline]
    pub fn request_initialize(
        &self,
        module: MmuModuleId,
        unk: u32,
        autoclear: bool,
    ) -> Result<MmuRequest, RequestInitializeError> {
        let id = cmif::request_initialize(self.0.handle(), module, unk, autoclear)?;
        Ok(MmuRequest { module, id })
    }

    /// Finalises a multimedia request (2.0.0+).
    #[inline]
    pub fn request_finalize(&self, request: &MmuRequest) -> Result<(), RequestFinalizeError> {
        cmif::request_finalize(self.0.handle(), request.id)
    }

    /// Sets the frequency in Hz and waits for the change to take
    /// effect (2.0.0+).
    #[inline]
    pub fn request_set_and_wait(
        &self,
        request: &MmuRequest,
        freq_hz: u32,
        timeout: i32,
    ) -> Result<(), RequestSetAndWaitError> {
        cmif::request_set_and_wait(self.0.handle(), request.id, freq_hz, timeout)
    }

    /// Gets the current frequency in Hz (2.0.0+).
    #[inline]
    pub fn request_get(&self, request: &MmuRequest) -> Result<u32, RequestGetError> {
        cmif::request_get(self.0.handle(), request.id)
    }
}

/// CMIF protocol methods (legacy, pre-2.0.0).
impl MmService {
    /// Initialises a multimedia request (legacy, pre-2.0.0).
    ///
    /// `unk` is ignored by official software.
    #[inline]
    pub fn request_initialize_legacy(
        &self,
        module: MmuModuleId,
        unk: u32,
        autoclear: bool,
    ) -> Result<MmuRequest, RequestInitializeError> {
        let id = cmif::request_initialize_legacy(self.0.handle(), module, unk, autoclear)?;
        Ok(MmuRequest { module, id })
    }

    /// Finalises a multimedia request (legacy, pre-2.0.0).
    #[inline]
    pub fn request_finalize_legacy(
        &self,
        request: &MmuRequest,
    ) -> Result<(), RequestFinalizeError> {
        cmif::request_finalize_legacy(self.0.handle(), request.module)
    }

    /// Sets the frequency in Hz and waits for the change to take
    /// effect (legacy, pre-2.0.0).
    #[inline]
    pub fn request_set_and_wait_legacy(
        &self,
        request: &MmuRequest,
        freq_hz: u32,
        timeout: i32,
    ) -> Result<(), RequestSetAndWaitError> {
        cmif::request_set_and_wait_legacy(self.0.handle(), request.module, freq_hz, timeout)
    }

    /// Gets the current frequency in Hz (legacy, pre-2.0.0).
    #[inline]
    pub fn request_get_legacy(&self, request: &MmuRequest) -> Result<u32, RequestGetError> {
        cmif::request_get_legacy(self.0.handle(), request.module)
    }
}

/// Connects to the `mm:u` (Multimedia) service using CMIF.
///
/// The caller must close the returned [`MmService`] when done.
pub fn connect_cmif(sm: &SmService) -> Result<MmService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(MmService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get mm:u service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
