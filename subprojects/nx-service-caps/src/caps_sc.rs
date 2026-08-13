//! Screenshot control (`caps:sc`) service implementation.
//!
//! Provides access to the screenshot control service for capturing raw RGBA8
//! and JPEG screenshots from the display.
//!
//! The service is connected once via [`connect_capssc_cmif`]; its methods are
//! then called directly, and the session is closed on drop.
//!
//! Callers choose which methods to call based on the target firmware version:
//! 2.0.0+ for the service itself, cmd 2 stubbed on 5.0.0+, cmds 1201-1203
//! require 3.0.0+ and debug mode, cmd 1204 requires 9.0.0+ with debug mode
//! before 10.0.0.

use nx_service_sm::SmService;
use nx_service_vi::ViLayerStack;
use nx_sf::service::{
    BorrowedSessionHandle,
    Session,
};

mod cmif;
mod proto;

pub use self::{
    cmif::{
        CaptureJpegError,
        CaptureRawImageError,
        CloseReadStreamError,
        OpenReadStreamError,
        ReadStreamError,
        ReadStreamInfo,
    },
    proto::CAPSSC_SERVICE_NAME,
};

/// Recommended JPEG output buffer size (512 KiB).
pub const JPEG_BUFFER_SIZE: usize = 0x80000;

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
    #[expect(
        clippy::too_many_arguments,
        reason = "the parameters are the command's own inputs plus the output buffer; grouping them into a \
                  struct would make the caller build a type before it can take a screenshot"
    )]
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
pub fn connect_capssc_cmif(sm: &SmService) -> Result<CapsscService, ConnectCapsscCmifError> {
    let handle = sm
        .get_service_handle_cmif(CAPSSC_SERVICE_NAME)
        .map_err(ConnectCapsscCmifError)?;

    let service = Session::new(handle, 0);

    Ok(CapsscService(service))
}

/// Error returned by [`connect_capssc_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get caps:sc service")]
pub struct ConnectCapsscCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
