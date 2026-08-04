//! Audio recorder service (`audrec:u`) implementation.
//!
//! Provides access to the audio final output recorder for capturing system
//! audio output into user-provided buffers.
//!
//! ## Architecture
//!
//! The service is non-domain. [`connect_cmif`] obtains the root session, then
//! [`AudrecService::open_final_output_recorder`] returns an
//! [`AudrecRecorder`] with its own independent session handle.
//!
//! ## Divergence from libnx
//!
//! libnx's `audrec.c` keeps a guarded global singleton managed by
//! `NX_GENERATE_SERVICE_GUARD`. This crate follows the convention of the
//! other `nx-service-*` crates: connect once via [`connect_cmif`], reuse the
//! service wrapper across calls, and let `Drop` close the session.
//!
//! Per IC-4, this crate is hosversion-unaware. Commands that differ across
//! firmware versions are exposed as paired methods:
//!
//! - [`AudrecRecorder::append_buffer`] / [`AudrecRecorder::append_buffer_legacy`]
//! - [`AudrecRecorder::get_released_buffers`] / [`AudrecRecorder::get_released_buffers_legacy`]

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::{
    ipc::Handle as RawSessionHandle,
    service::{
        BorrowedSessionHandle,
        DispatchError,
        OwnedSessionHandle,
        Session,
    },
};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::{
        AppendBufferError,
        GetReleasedBuffersError,
        OpenRecorderError,
    },
    proto::SERVICE_NAME,
    types::{
        FinalOutputRecorderBuffer,
        FinalOutputRecorderParameter,
        FinalOutputRecorderParameterInternal,
    },
};

/// Audio recorder (`audrec:u`) root session wrapper.
///
/// Use [`open_final_output_recorder`](Self::open_final_output_recorder) to
/// create a recorder sub-object.
#[repr(transparent)]
pub struct AudrecService(Session);

impl AudrecService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    /// Opens a final output recorder.
    ///
    /// `param` specifies the desired sample rate and channel count; `aruid`
    /// is the application resource user ID. Returns the recorder wrapper and
    /// the negotiated internal parameters.
    pub fn open_final_output_recorder(
        &self,
        param: &FinalOutputRecorderParameter,
        aruid: u64,
    ) -> Result<(AudrecRecorder, FinalOutputRecorderParameterInternal), OpenRecorderError> {
        let input = types::OpenRecorderIn {
            param: *param,
            aruid,
        };
        let (raw_handle, param_out) = cmif::open_final_output_recorder(&self.0, &input)?;

        // SAFETY: the kernel returned a valid move handle for the new recorder
        // session; ownership transfers to the new `Session`.
        let handle = OwnedSessionHandle::from_handle_unchecked(
            RawSessionHandle::from_raw_unchecked(raw_handle),
        );
        Ok((AudrecRecorder(Session::new(handle, 0)), param_out))
    }
}

/// Final output recorder session wrapper.
///
/// Obtained via [`AudrecService::open_final_output_recorder`]. Owns its own
/// independent session handle.
#[repr(transparent)]
pub struct AudrecRecorder(Session);

impl AudrecRecorder {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods for `AudrecRecorder`.
impl AudrecRecorder {
    /// Starts recording.
    #[inline]
    pub fn start(&self) -> Result<(), DispatchError> {
        cmif::recorder_start(&self.0)
    }

    /// Stops recording.
    #[inline]
    pub fn stop(&self) -> Result<(), DispatchError> {
        cmif::recorder_stop(&self.0)
    }

    /// Registers the buffer event and returns a copy-handle for the event.
    ///
    /// The returned handle can be used with event-waiting primitives to know
    /// when a buffer has been filled and released.
    #[inline]
    pub fn register_buffer_event(&self) -> Result<u32, DispatchError> {
        cmif::recorder_register_buffer_event(&self.0)
    }

    /// Appends a final output recorder buffer (auto-select). \[3.0.0+\]
    ///
    /// `buffer_client_ptr` is the client-side pointer identifying this buffer.
    /// `param` describes the buffer layout.
    #[inline]
    pub fn append_buffer(
        &self,
        buffer_client_ptr: u64,
        param: &FinalOutputRecorderBuffer,
    ) -> Result<(), AppendBufferError> {
        cmif::recorder_append_buffer(&self.0, buffer_client_ptr, param)
    }

    /// Appends a final output recorder buffer (map-alias, legacy). \[1.0.0-2.x.x\]
    ///
    /// `buffer_client_ptr` is the client-side pointer identifying this buffer.
    /// `param` describes the buffer layout.
    #[inline]
    pub fn append_buffer_legacy(
        &self,
        buffer_client_ptr: u64,
        param: &FinalOutputRecorderBuffer,
    ) -> Result<(), AppendBufferError> {
        cmif::recorder_append_buffer_legacy(&self.0, buffer_client_ptr, param)
    }

    /// Gets released final output recorder buffers (auto-select). \[3.0.0+\]
    ///
    /// Fills `out_buffers` with the client-side pointers of released buffers.
    /// Returns `(count, released)` where `count` is the number of buffer
    /// pointers written and `released` is a release counter.
    #[inline]
    pub fn get_released_buffers(
        &self,
        out_buffers: &mut [u64],
    ) -> Result<(u32, u64), GetReleasedBuffersError> {
        cmif::recorder_get_released_buffers(&self.0, out_buffers)
    }

    /// Gets released final output recorder buffers (map-alias, legacy). \[1.0.0-2.x.x\]
    ///
    /// Fills `out_buffers` with the client-side pointers of released buffers.
    /// Returns `(count, released)` where `count` is the number of buffer
    /// pointers written and `released` is a release counter.
    #[inline]
    pub fn get_released_buffers_legacy(
        &self,
        out_buffers: &mut [u64],
    ) -> Result<(u32, u64), GetReleasedBuffersError> {
        cmif::recorder_get_released_buffers_legacy(&self.0, out_buffers)
    }
}

/// Connects to the `audrec:u` service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<AudrecService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    Ok(AudrecService(Session::new(handle, 0)))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get audrec:u service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
