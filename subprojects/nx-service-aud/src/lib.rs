//! Audio service (`aud:a`, `aud:d`) implementation.
//!
//! Provides two interfaces for audio system management:
//!
//! - **`aud:a`** (admin): Suspend/resume audio for a process, get/set master
//!   and record volumes per process.
//! - **`aud:d`** (debug): Suspend/resume audio for a process (debug variant).
//!
//! ## Divergence from libnx
//!
//! libnx's `aud.c` keeps guarded global singletons (`g_audaSrv`,
//! `g_auddSrv`) managed by `NX_GENERATE_SERVICE_GUARD` and enforces a
//! hosversion 11.0.0+ check at initialization. This crate follows the
//! convention of the other `nx-service-*` crates: connect once via
//! [`connect_auda_cmif`] or [`connect_audd_cmif`], reuse the service
//! wrapper across calls, and let `Drop` close the session.
//!
//! Per IC-4, this crate is hosversion-unaware — callers choose when to
//! connect based on the target firmware version (11.0.0+ for both services).

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{
    BorrowedSessionHandle,
    Session,
};

mod cmif;
mod proto;

pub use self::{
    cmif::{
        GetVolumeError,
        SetVolumeError,
        SuspendResumeError,
    },
    proto::{
        AUDA_SERVICE_NAME,
        AUDD_SERVICE_NAME,
    },
};

/// Audio admin (`aud:a`) session wrapper.
///
/// Provides suspend/resume and volume control for audio processes.
#[repr(transparent)]
pub struct AudaService(Session);

impl AudaService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods for `aud:a`.
impl AudaService {
    /// Suspends audio for a process.
    #[inline]
    pub fn request_suspend_audio(&self, pid: u64, delay: u64) -> Result<(), SuspendResumeError> {
        cmif::request_suspend_audio(self.0.handle(), pid, delay)
    }

    /// Resumes audio for a process.
    #[inline]
    pub fn request_resume_audio(&self, pid: u64, delay: u64) -> Result<(), SuspendResumeError> {
        cmif::request_resume_audio(self.0.handle(), pid, delay)
    }

    /// Gets the master volume for a process's audio output.
    #[inline]
    pub fn get_audio_output_process_master_volume(&self, pid: u64) -> Result<f32, GetVolumeError> {
        cmif::get_audio_output_process_master_volume(self.0.handle(), pid)
    }

    /// Sets the master volume for a process's audio output.
    #[inline]
    pub fn set_audio_output_process_master_volume(
        &self,
        pid: u64,
        delay: u64,
        volume: f32,
    ) -> Result<(), SetVolumeError> {
        cmif::set_audio_output_process_master_volume(self.0.handle(), pid, delay, volume)
    }

    /// Gets the master volume for a process's audio input.
    #[inline]
    pub fn get_audio_input_process_master_volume(&self, pid: u64) -> Result<f32, GetVolumeError> {
        cmif::get_audio_input_process_master_volume(self.0.handle(), pid)
    }

    /// Sets the master volume for a process's audio input and output.
    #[inline]
    pub fn set_audio_input_process_master_volume(
        &self,
        pid: u64,
        delay: u64,
        volume: f32,
    ) -> Result<(), SetVolumeError> {
        cmif::set_audio_input_process_master_volume(self.0.handle(), pid, delay, volume)
    }

    /// Gets the record volume for a process's audio output.
    #[inline]
    pub fn get_audio_output_process_record_volume(&self, pid: u64) -> Result<f32, GetVolumeError> {
        cmif::get_audio_output_process_record_volume(self.0.handle(), pid)
    }

    /// Sets the record volume for a process's audio output.
    #[inline]
    pub fn set_audio_output_process_record_volume(
        &self,
        pid: u64,
        delay: u64,
        volume: f32,
    ) -> Result<(), SetVolumeError> {
        cmif::set_audio_output_process_record_volume(self.0.handle(), pid, delay, volume)
    }
}

/// Audio debug (`aud:d`) session wrapper.
///
/// Provides debug suspend/resume control for audio processes.
#[repr(transparent)]
pub struct AuddService(Session);

impl AuddService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods for `aud:d`.
impl AuddService {
    /// Suspends audio for a process (debug).
    #[inline]
    pub fn request_suspend_audio_for_debug(
        &self,
        pid: u64,
        delay: u64,
    ) -> Result<(), SuspendResumeError> {
        cmif::request_suspend_audio_for_debug(self.0.handle(), pid, delay)
    }

    /// Resumes audio for a process (debug).
    #[inline]
    pub fn request_resume_audio_for_debug(
        &self,
        pid: u64,
        delay: u64,
    ) -> Result<(), SuspendResumeError> {
        cmif::request_resume_audio_for_debug(self.0.handle(), pid, delay)
    }
}

/// Connects to the `aud:a` (Audio Admin) service using CMIF.
pub fn connect_auda_cmif(sm: &SmService) -> Result<AudaService, ConnectAudaCmifError> {
    let handle = sm
        .get_service_handle_cmif(AUDA_SERVICE_NAME)
        .map_err(ConnectAudaCmifError)?;

    let service = Session::new(handle, 0);

    Ok(AudaService(service))
}

/// Error returned by [`connect_auda_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get aud:a service")]
pub struct ConnectAudaCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

/// Connects to the `aud:d` (Audio Debug) service using CMIF.
pub fn connect_audd_cmif(sm: &SmService) -> Result<AuddService, ConnectAuddCmifError> {
    let handle = sm
        .get_service_handle_cmif(AUDD_SERVICE_NAME)
        .map_err(ConnectAuddCmifError)?;

    let service = Session::new(handle, 0);

    Ok(AuddService(service))
}

/// Error returned by [`connect_audd_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get aud:d service")]
pub struct ConnectAuddCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
