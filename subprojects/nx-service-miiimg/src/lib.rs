//! Mii image (`miiimg`) service implementation.
//!
//! Provides access to the Mii image database for querying and loading
//! Mii profile images as raw RGBA8 data.
//!
//! ## Divergence from libnx
//!
//! libnx's `miiimg.c` keeps a guarded global singleton (`g_miiimgSrv`) managed
//! by `NX_GENERATE_SERVICE_GUARD`, and calls the initialize command (cmd 0)
//! automatically during `miiimgInitialize`. This crate follows the convention of
//! the other `nx-service-*` crates: connect once via [`connect_cmif`], then call
//! [`MiiimgService::initialize`] explicitly before using the database.
//!
//! Per IC-4, this crate is hosversion-unaware — callers choose when to use
//! this service based on the target firmware version (5.0.0+).

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{BorrowedSessionHandle, Session};

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{
        GetAttributeError, GetCountError, InitializeError, IsEmptyError, IsFullError,
        LoadImageError, ReloadError,
    },
    proto::SERVICE_NAME,
    types::{MiiCreateId, MiiimgImageAttribute, MiiimgImageId},
};

/// Mii image database service wrapper.
#[repr(transparent)]
pub struct MiiimgService(Session);

impl MiiimgService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl MiiimgService {
    /// Initializes the image database.
    ///
    /// Must be called after [`connect_cmif`] and before other database
    /// operations. The `mode` parameter is passed to the service (libnx
    /// uses `1`).
    #[inline]
    pub fn initialize(&self, mode: u8) -> Result<u8, InitializeError> {
        cmif::initialize(self.0.handle(), mode)
    }

    /// Reloads the image database.
    #[inline]
    pub fn reload(&self) -> Result<(), ReloadError> {
        cmif::reload(self.0.handle())
    }

    /// Gets the number of mii images in the database.
    #[inline]
    pub fn get_count(&self) -> Result<i32, GetCountError> {
        cmif::get_count(self.0.handle())
    }

    /// Gets whether the image database is empty.
    #[inline]
    pub fn is_empty(&self) -> Result<bool, IsEmptyError> {
        cmif::is_empty(self.0.handle())
    }

    /// Gets whether the image database is full.
    #[inline]
    pub fn is_full(&self) -> Result<bool, IsFullError> {
        cmif::is_full(self.0.handle())
    }

    /// Gets the image attribute at the specified index.
    #[inline]
    pub fn get_attribute(&self, index: i32) -> Result<MiiimgImageAttribute, GetAttributeError> {
        cmif::get_attribute(self.0.handle(), index)
    }

    /// Loads the image data (raw RGBA8) for the specified image ID.
    ///
    /// The optimal buffer size is `0x40000` (256 KiB).
    #[inline]
    pub fn load_image(&self, id: MiiimgImageId, dst: &mut [u8]) -> Result<(), LoadImageError> {
        cmif::load_image(self.0.handle(), id, dst)
    }
}

/// Connects to the mii image service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<MiiimgService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(MiiimgService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get miiimg service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
