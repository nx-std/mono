//! JPEG decoder (`caps:dc`) service implementation.
//!
//! Provides access to the capture JPEG decoder for decoding JPEG buffers
//! into RGBA8 images and shrinking JPEG images.
//!
//! The service is connected once via [`connect_capsdc_cmif`]; its methods are
//! then called directly, and the session is closed on drop.
//!
//! Callers choose which methods to call based on the target firmware version:
//! 4.0.0+ for the service itself, 17.0.0+ for [`CapsdcService::shrink_jpeg`],
//! 19.0.0+ for [`CapsdcService::shrink_jpeg_ex`].

use nx_service_sm::SmService;
use nx_sf::service::{
    BorrowedSessionHandle,
    Session,
};

mod cmif;
mod proto;

pub use self::{
    cmif::{
        DecodeJpegError,
        ShrinkJpegError,
        ShrinkJpegExError,
    },
    proto::CAPSDC_SERVICE_NAME,
};
use crate::screenshot::ScreenShotDecodeOption;

/// JPEG decoder service wrapper.
#[repr(transparent)]
pub struct CapsdcService(Session);

impl CapsdcService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl CapsdcService {
    /// Decodes a JPEG buffer into RGBA8.
    ///
    /// `out_image` should be at least `width * height * 4` bytes.
    #[inline]
    pub fn decode_jpeg(
        &self,
        width: u32,
        height: u32,
        opts: &ScreenShotDecodeOption,
        jpeg: &[u8],
        out_image: &mut [u8],
    ) -> Result<(), DecodeJpegError> {
        cmif::decode_jpeg(self.0.handle(), width, height, opts, jpeg, out_image)
    }

    /// Shrinks a JPEG's dimensions by 2, auto-selecting compression quality.
    ///
    /// Returns the size of the resulting JPEG in the output buffer.
    #[inline]
    pub fn shrink_jpeg(
        &self,
        width: u32,
        height: u32,
        opts: &ScreenShotDecodeOption,
        jpeg: &[u8],
        out_jpeg: &mut [u8],
    ) -> Result<u64, ShrinkJpegError> {
        cmif::shrink_jpeg(self.0.handle(), width, height, opts, jpeg, out_jpeg)
    }

    /// Shrinks a JPEG with explicit target dimensions and quality.
    ///
    /// `jpeg_quality` must be in the range 0–100. Returns the size of the
    /// resulting JPEG in the output buffer.
    #[inline]
    pub fn shrink_jpeg_ex(
        &self,
        scaled_width: u32,
        scaled_height: u32,
        jpeg_quality: u32,
        opts: &ScreenShotDecodeOption,
        jpeg: &[u8],
        out_jpeg: &mut [u8],
    ) -> Result<u64, ShrinkJpegExError> {
        cmif::shrink_jpeg_ex(
            self.0.handle(),
            scaled_width,
            scaled_height,
            jpeg_quality,
            opts,
            jpeg,
            out_jpeg,
        )
    }
}

/// Connects to the JPEG decoder service using CMIF.
pub fn connect_capsdc_cmif(sm: &SmService) -> Result<CapsdcService, ConnectCapsdcCmifError> {
    let handle = sm
        .get_service_handle_cmif(CAPSDC_SERVICE_NAME)
        .map_err(ConnectCapsdcCmifError)?;

    let service = Session::new(handle, 0);

    Ok(CapsdcService(service))
}

/// Error returned by [`connect_capsdc_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get caps:dc service")]
pub struct ConnectCapsdcCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
