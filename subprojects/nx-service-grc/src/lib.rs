//! GRC game recording service (`grc:*`) implementation.
//!
//! Provides access to game recording functionality on the Nintendo Switch.
//!
//! ## Architecture
//!
//! - **`grc:d`** (debug): Root session for retrieving continuous recording
//!   stream data. [`connect_cmif`] obtains the root session.
//! - **[`GrcGameMovieTrimmer`]**: Wraps an IGameMovieTrimmer session obtained
//!   from the applet service (`appletCreateGameMovieTrimmer`). Provides
//!   commands for trimming recorded game movies.
//! - **[`GrcMovieMaker`]**: Wraps an IMovieMaker session obtained from the
//!   applet service (`appletCreateMovieMaker` → GetGrcMovieMaker cmd 0).
//!   Provides commands for offscreen recording.
//!
//! ## Divergence from libnx
//!
//! libnx's `grc.c` uses a guarded global singleton for `grc:d`, provides
//! high-level convenience wrappers that manage applet interactions, transfer
//! memory, and event waiting internally. This crate exposes the raw IPC
//! commands and lets the caller manage the session lifecycle, applet
//! sub-objects, and hosversion selection per IC-4.

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

pub use nx_service_caps::ApplicationAlbumEntry;

pub use self::{
    cmif::{
        CompleteFinishEx1Error,
        CreateVideoProxyError,
        TransferError,
    },
    proto::SERVICE_NAME,
    types::{
        GameMovieId,
        GrcStream,
        OffscreenRecordingParameter,
        TransferResult,
    },
};

// ---------------------------------------------------------------------------
// grc:d root service
// ---------------------------------------------------------------------------

/// Game recording debug service (`grc:d`) root session wrapper.
#[repr(transparent)]
pub struct GrcdService(Session);

impl GrcdService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// grc:d IPC commands.
impl GrcdService {
    /// Begins streaming (cmd 1).
    ///
    /// Must not be called more than once, even from a different session.
    #[inline]
    pub fn begin(&self) -> Result<(), DispatchError> {
        cmif::begin(&self.0)
    }

    /// Retrieves stream data from the continuous recorder (cmd 2).
    ///
    /// Blocks until data is available. Hangs if no application with video
    /// capture enabled is running.
    #[inline]
    pub fn transfer(
        &self,
        stream: GrcStream,
        buffer: &mut [u8],
    ) -> Result<TransferResult, TransferError> {
        cmif::transfer(&self.0, stream as u32, buffer.as_mut_ptr(), buffer.len())
    }
}

// ---------------------------------------------------------------------------
// IGameMovieTrimmer sub-object
// ---------------------------------------------------------------------------

/// Game movie trimmer session wrapper (IGameMovieTrimmer).
///
/// Obtained from the applet service via `appletCreateGameMovieTrimmer`.
/// Callers wrap the resulting session handle with
/// [`GrcGameMovieTrimmer::from_raw_unchecked`].
#[repr(transparent)]
pub struct GrcGameMovieTrimmer(Session);

impl GrcGameMovieTrimmer {
    /// Adopts a pre-obtained IGameMovieTrimmer session handle.
    ///
    /// The caller must ensure `handle` names a live IGameMovieTrimmer session this process
    /// owns and that nothing else will close, since the returned value closes it on drop.
    /// Neither half can be checked here: only the kernel knows which handle numbers are live,
    /// and only the server knows which object one addresses. Breaking either costs a rejected
    /// request or a close against a number the kernel has since reused, not a fault.
    #[inline]
    pub fn from_raw_unchecked(handle: u32) -> Self {
        // SAFETY: Both halves are delegated to this constructor's precondition, which is the
        // boundary at which the caller vouches for a handle it obtained from the applet
        // service's `CreateGameMovieTrimmer`.
        Self(Session::new(
            OwnedSessionHandle::from_handle_unchecked(RawSessionHandle::from_raw_unchecked(handle)),
            0,
        ))
    }

    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// IGameMovieTrimmer IPC commands.
impl GrcGameMovieTrimmer {
    /// Begins trimming a game movie (cmd 1).
    ///
    /// `start` and `end` are timestamps in 0.5 s units.
    #[inline]
    pub fn begin_trim(&self, id: &GameMovieId, start: i32, end: i32) -> Result<(), DispatchError> {
        cmif::trimmer_begin_trim(&self.0, id, start, end)
    }

    /// Ends trimming and retrieves the output movie ID (cmd 2).
    #[inline]
    pub fn end_trim(&self) -> Result<GameMovieId, DispatchError> {
        cmif::trimmer_end_trim(&self.0)
    }

    /// Gets the "not trimming" event handle (cmd 10, copy handle,
    /// autoclear=false).
    #[inline]
    pub fn get_not_trimming_event(&self) -> Result<u32, DispatchError> {
        cmif::trimmer_get_not_trimming_event(&self.0)
    }

    /// Sets the thumbnail RGBA image for the trimmed movie (cmd 20).
    ///
    /// `buffer` is RGBA8 pixel data (typically 1280×720).
    #[inline]
    pub fn set_thumbnail_rgba(
        &self,
        buffer: &[u8],
        width: i32,
        height: i32,
    ) -> Result<(), DispatchError> {
        cmif::trimmer_set_thumbnail_rgba(&self.0, buffer.as_ptr(), buffer.len(), width, height)
    }
}

// ---------------------------------------------------------------------------
// IMovieMaker sub-object
// ---------------------------------------------------------------------------

/// Movie maker session wrapper (IMovieMaker).
///
/// Obtained by calling GetGrcMovieMaker (cmd 0) on the applet's IMovieMaker
/// object. Callers wrap the resulting session handle with
/// [`GrcMovieMaker::from_raw_unchecked`].
#[repr(transparent)]
pub struct GrcMovieMaker(Session);

impl GrcMovieMaker {
    /// Adopts a pre-obtained IMovieMaker session handle.
    ///
    /// The caller carries the same obligation as
    /// [`GrcGameMovieTrimmer::from_raw_unchecked`], for a grc IMovieMaker session rather than
    /// an IGameMovieTrimmer one, and breaking it costs the same.
    #[inline]
    pub fn from_raw_unchecked(handle: u32) -> Self {
        // SAFETY: Both halves are delegated to this constructor's precondition, which is the
        // boundary at which the caller vouches for a handle it obtained from GetGrcMovieMaker.
        Self(Session::new(
            OwnedSessionHandle::from_handle_unchecked(RawSessionHandle::from_raw_unchecked(handle)),
            0,
        ))
    }

    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// IMovieMaker IPC commands.
impl GrcMovieMaker {
    /// Creates a video proxy sub-object (cmd 2).
    ///
    /// Returns the raw session handle for the IHOSBinderDriver video proxy.
    #[inline]
    pub fn create_video_proxy(&self) -> Result<u32, CreateVideoProxyError> {
        cmif::maker_create_video_proxy(&self.0)
    }

    /// Sets the album shim library version (cmd 9). \[7.0.0+\]
    #[inline]
    pub fn set_album_shim_library_version(&self, version: u64) -> Result<(), DispatchError> {
        cmif::maker_set_album_shim_library_version(&self.0, version)
    }

    /// Opens an offscreen layer (cmd 10). Returns the binder ID.
    #[inline]
    pub fn open_offscreen_layer(&self, layer_handle: u64) -> Result<u32, DispatchError> {
        cmif::maker_open_offscreen_layer(&self.0, layer_handle)
    }

    /// Closes an offscreen layer (cmd 11).
    #[inline]
    pub fn close_offscreen_layer(&self, layer_handle: u64) -> Result<(), DispatchError> {
        cmif::maker_close_offscreen_layer(&self.0, layer_handle)
    }

    /// Aborts offscreen recording (cmd 21).
    #[inline]
    pub fn abort_offscreen_recording(&self, layer_handle: u64) -> Result<(), DispatchError> {
        cmif::maker_abort_offscreen_recording(&self.0, layer_handle)
    }

    /// Requests offscreen recording finish ready (cmd 22).
    #[inline]
    pub fn request_offscreen_recording_finish_ready(
        &self,
        layer_handle: u64,
    ) -> Result<(), DispatchError> {
        cmif::maker_request_offscreen_recording_finish_ready(&self.0, layer_handle)
    }

    /// Starts offscreen recording (cmd 24).
    #[inline]
    pub fn start_offscreen_recording(
        &self,
        layer_handle: u64,
        param: &OffscreenRecordingParameter,
    ) -> Result<(), DispatchError> {
        cmif::maker_start_offscreen_recording(&self.0, layer_handle, param)
    }

    /// Completes offscreen recording finish (pre-7.0.0, cmd 25).
    #[inline]
    pub fn complete_offscreen_recording_finish_ex0(
        &self,
        layer_handle: u64,
        width: i32,
        height: i32,
        userdata: &[u8],
        thumbnail: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::maker_complete_offscreen_recording_finish_ex0(
            &self.0,
            layer_handle,
            width,
            height,
            userdata.as_ptr(),
            userdata.len(),
            thumbnail.as_ptr(),
            thumbnail.len(),
        )
    }

    /// Completes offscreen recording finish (7.0.0+, cmd 26).
    ///
    /// Returns the [`ApplicationAlbumEntry`] for the recorded video.
    #[inline]
    pub fn complete_offscreen_recording_finish_ex1(
        &self,
        layer_handle: u64,
        width: i32,
        height: i32,
        userdata: &[u8],
        thumbnail: &[u8],
    ) -> Result<ApplicationAlbumEntry, CompleteFinishEx1Error> {
        cmif::maker_complete_offscreen_recording_finish_ex1(
            &self.0,
            layer_handle,
            width,
            height,
            userdata.as_ptr(),
            userdata.len(),
            thumbnail.as_ptr(),
            thumbnail.len(),
        )
    }

    /// Gets the offscreen layer error (cmd 30).
    #[inline]
    pub fn get_offscreen_layer_error(&self, layer_handle: u64) -> Result<(), DispatchError> {
        cmif::maker_get_offscreen_layer_error(&self.0, layer_handle)
    }

    /// Encodes offscreen layer audio sample data (cmd 41).
    ///
    /// Returns the number of bytes consumed from the buffer.
    #[inline]
    pub fn encode_offscreen_layer_audio_sample(
        &self,
        layer_handle: u64,
        buffer: &[u8],
    ) -> Result<u64, DispatchError> {
        cmif::maker_encode_offscreen_layer_audio_sample(
            &self.0,
            layer_handle,
            buffer.as_ptr(),
            buffer.len(),
        )
    }

    /// Gets the offscreen layer recording finish ready event handle
    /// (cmd 50, copy handle, autoclear=false).
    #[inline]
    pub fn get_offscreen_layer_recording_finish_ready_event(
        &self,
        layer_handle: u64,
    ) -> Result<u32, DispatchError> {
        cmif::maker_get_offscreen_layer_recording_finish_ready_event(&self.0, layer_handle)
    }

    /// Gets the offscreen layer audio encode ready event handle
    /// (cmd 52, copy handle, autoclear=false).
    #[inline]
    pub fn get_offscreen_layer_audio_encode_ready_event(
        &self,
        layer_handle: u64,
    ) -> Result<u32, DispatchError> {
        cmif::maker_get_offscreen_layer_audio_encode_ready_event(&self.0, layer_handle)
    }
}

// ---------------------------------------------------------------------------
// Connect function
// ---------------------------------------------------------------------------

/// Connects to the `grc:d` service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<GrcdService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(GrcdService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get grc:d service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
