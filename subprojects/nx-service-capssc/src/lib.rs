//! Screenshot control (`caps:sc`) service implementation.
//!
//! Provides access to the screenshot control service for capturing raw RGBA8
//! and JPEG screenshots from the display.
//!
//! ## Divergence from libnx
//!
//! libnx's `capssc.c` keeps a guarded global singleton (`g_capsscSrv`) managed
//! by `NX_GENERATE_SERVICE_GUARD`, and enforces hosversion checks at each call
//! site. This crate follows the convention of the other `nx-service-*` crates:
//! connect once via [`connect_cmif`], then call methods directly.
//!
//! Per IC-4, this crate is hosversion-unaware — callers choose which methods
//! to call based on the target firmware version (2.0.0+ for the service itself,
//! cmd 2 stubbed on 5.0.0+, cmds 1201–1203 require 3.0.0+ and debug mode,
//! cmd 1204 requires 9.0.0+ with debug mode before 10.0.0).

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_service_vi::ViLayerStack;
use nx_sf::service::{BorrowedSessionHandle, Session};

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{
        CaptureJpegError, CaptureRawImageError, CloseReadStreamError, OpenReadStreamError,
        ReadStreamError, ReadStreamInfo,
    },
    proto::SERVICE_NAME,
    types::JPEG_BUFFER_SIZE,
};

/// Screenshot control service wrapper.
#[repr(transparent)]
pub struct CapsscService(Session);

impl CapsscService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl CapsscService {
    /// Captures a raw RGBA8 screenshot with a timeout.
    ///
    /// `out_image` should be at least `1280 * 720 * 4 * buffer_count` bytes.
    /// Not available on 5.0.0+ (stubbed).
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn capture_raw_image_with_timeout(
        &self,
        layer_stack: ViLayerStack,
        width: u64,
        height: u64,
        buffer_count: i64,
        buffer_index: i64,
        timeout: i64,
        out_image: &mut [u8],
    ) -> Result<(), CaptureRawImageError> {
        cmif::capture_raw_image_with_timeout(
            self.0.handle(),
            layer_stack,
            width,
            height,
            buffer_count,
            buffer_index,
            timeout,
            out_image,
        )
    }

    /// Opens a raw screenshot read stream.
    ///
    /// The stream must be closed with [`close_raw_screen_shot_read_stream`](Self::close_raw_screen_shot_read_stream).
    /// Requires 3.0.0+ and debug mode.
    #[inline]
    pub fn open_raw_screen_shot_read_stream(
        &self,
        layer_stack: ViLayerStack,
        timeout: i64,
    ) -> Result<ReadStreamInfo, OpenReadStreamError> {
        cmif::open_raw_screen_shot_read_stream(self.0.handle(), layer_stack, timeout)
    }

    /// Closes a raw screenshot read stream opened by [`open_raw_screen_shot_read_stream`](Self::open_raw_screen_shot_read_stream).
    ///
    /// Requires 3.0.0+ and debug mode.
    #[inline]
    pub fn close_raw_screen_shot_read_stream(&self) -> Result<(), CloseReadStreamError> {
        cmif::close_raw_screen_shot_read_stream(self.0.handle())
    }

    /// Reads from a raw screenshot read stream.
    ///
    /// Returns the number of bytes written to the output buffer.
    /// Requires 3.0.0+ and debug mode.
    #[inline]
    pub fn read_raw_screen_shot_read_stream(
        &self,
        offset: u64,
        out_buf: &mut [u8],
    ) -> Result<u64, ReadStreamError> {
        cmif::read_raw_screen_shot_read_stream(self.0.handle(), offset, out_buf)
    }

    /// Captures a JPEG screenshot.
    ///
    /// Returns the size of the captured JPEG in the output buffer.
    /// Requires 9.0.0+; debug mode required before 10.0.0.
    #[inline]
    pub fn capture_jpeg_screen_shot(
        &self,
        layer_stack: ViLayerStack,
        timeout: i64,
        out_jpeg: &mut [u8],
    ) -> Result<u64, CaptureJpegError> {
        cmif::capture_jpeg_screen_shot(self.0.handle(), layer_stack, timeout, out_jpeg)
    }
}

/// Connects to the screenshot control service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<CapsscService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(CapsscService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get caps:sc service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
