//! Album control (`caps:c`) service implementation.
//!
//! Provides access to the album control service for managing album storage
//! availability, applet registration, screenshot saving, overlay thumbnails,
//! and movie read/write streams.
//!
//! ## Architecture
//!
//! The service is non-domain. [`connect_cmif`] obtains the root session.
//! Movie streaming (both read and write) requires a control session
//! sub-object obtained via [`CapscService::open_control_session`], which
//! returns a [`CapscControlSession`] with its own independent session handle.
//!
//! ## Divergence from libnx
//!
//! libnx's `capsc.c` keeps two guarded global singletons (`g_capscSrv` and
//! `g_capscControl`) managed by `NX_GENERATE_SERVICE_GUARD`, enforces
//! hosversion checks at each call site, and automatically calls
//! `SetShimLibraryVersion` during initialization on 7.0.0+. This crate
//! follows the convention of the other `nx-service-*` crates: connect once
//! via [`connect_cmif`], then call methods directly.
//!
//! Per IC-4, this crate is hosversion-unaware — callers choose which methods
//! to call based on the target firmware version. Commands that have different
//! wire formats across hosversions are exposed as paired methods (e.g.,
//! `register_applet_resource_user_id_legacy` / `register_applet_resource_user_id`).

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_caps::{
    AlbumEntry,
    AlbumFileId,
    ApplicationAlbumEntry,
    ScreenShotAttribute,
};
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
        OpenControlSessionError,
        ReadStreamDataError,
        SaveScreenShotError,
        SetOverlayThumbnailError,
        WriteStreamDataError,
    },
    proto::SERVICE_NAME,
    types::CapsApplicationId,
};

/// Album control (`caps:c`) root session wrapper.
///
/// Use the storage notification, registration, and screenshot methods on
/// this wrapper. For movie read/write streaming, open a
/// [`CapscControlSession`] via [`open_control_session`](Self::open_control_session).
#[repr(transparent)]
pub struct CapscService(Session);

impl CapscService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods for `CapscService`.
impl CapscService {
    /// Sets the shim library version (cmd 33). Should be called after connect on 7.0.0+.
    #[inline]
    pub fn set_shim_library_version(
        &self,
        version: u64,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::set_shim_library_version(&self.0, version, applet_resource_user_id)
    }

    /// Notifies that an album storage is available (cmd 2001).
    ///
    /// Capsrv will mount the image directory on the specified storage.
    #[inline]
    pub fn notify_album_storage_is_available(&self, storage: u8) -> Result<(), DispatchError> {
        cmif::notify_album_storage_is_available(&self.0, storage)
    }

    /// Notifies that an album storage is unavailable (cmd 2002).
    ///
    /// Capsrv will unmount the image directory on the specified storage.
    #[inline]
    pub fn notify_album_storage_is_unavailable(&self, storage: u8) -> Result<(), DispatchError> {
        cmif::notify_album_storage_is_unavailable(&self.0, storage)
    }

    /// Registers an applet resource user ID, legacy wire format (pre-19.0.0, cmd 2011).
    ///
    /// \[2.0.0+\]
    #[inline]
    pub fn register_applet_resource_user_id_legacy(
        &self,
        applet_resource_user_id: u64,
        application_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::register_applet_resource_user_id_legacy(
            &self.0,
            applet_resource_user_id,
            application_id,
        )
    }

    /// Registers an applet resource user ID (19.0.0+, cmd 2011).
    #[inline]
    pub fn register_applet_resource_user_id(
        &self,
        applet_resource_user_id: u64,
        application_id: &CapsApplicationId,
    ) -> Result<(), DispatchError> {
        cmif::register_applet_resource_user_id(&self.0, applet_resource_user_id, application_id)
    }

    /// Unregisters an applet resource user ID, legacy wire format (pre-19.0.0, cmd 2012).
    ///
    /// \[2.0.0+\]
    #[inline]
    pub fn unregister_applet_resource_user_id_legacy(
        &self,
        applet_resource_user_id: u64,
        application_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::unregister_applet_resource_user_id_legacy(
            &self.0,
            applet_resource_user_id,
            application_id,
        )
    }

    /// Unregisters an applet resource user ID (19.0.0+, cmd 2012).
    #[inline]
    pub fn unregister_applet_resource_user_id(
        &self,
        applet_resource_user_id: u64,
        application_id: &CapsApplicationId,
    ) -> Result<(), DispatchError> {
        cmif::unregister_applet_resource_user_id(&self.0, applet_resource_user_id, application_id)
    }

    /// Gets the application ID from an ARUID, legacy wire format (pre-19.0.0, cmd 2013).
    ///
    /// Returns the application ID as a bare `u64`. \[2.0.0+\]
    #[inline]
    pub fn get_application_id_from_aruid_legacy(&self, aruid: u64) -> Result<u64, DispatchError> {
        cmif::get_application_id_from_aruid_legacy(&self.0, aruid)
    }

    /// Gets the application ID from an ARUID (19.0.0+, cmd 2013).
    #[inline]
    pub fn get_application_id_from_aruid(
        &self,
        aruid: u64,
    ) -> Result<CapsApplicationId, DispatchError> {
        cmif::get_application_id_from_aruid(&self.0, aruid)
    }

    /// Checks whether an application ID is registered (cmd 2014). \[2.0.0+\]
    #[inline]
    pub fn check_application_id_registered(
        &self,
        application_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::check_application_id_registered(&self.0, application_id)
    }

    /// Generates a current album file ID, legacy wire format (pre-19.0.0, cmd 2101).
    ///
    /// \[2.0.0+\]
    #[inline]
    pub fn generate_current_album_file_id_legacy(
        &self,
        contents: u8,
        application_id: u64,
    ) -> Result<AlbumFileId, DispatchError> {
        cmif::generate_current_album_file_id_legacy(&self.0, contents, application_id)
    }

    /// Generates a current album file ID (19.0.0+, cmd 2101).
    #[inline]
    pub fn generate_current_album_file_id(
        &self,
        contents: u8,
        application_id: &CapsApplicationId,
    ) -> Result<AlbumFileId, DispatchError> {
        cmif::generate_current_album_file_id(&self.0, contents, application_id)
    }

    /// Generates an application album entry (cmd 2102). \[2.0.0+\]
    #[inline]
    pub fn generate_application_album_entry(
        &self,
        entry: &AlbumEntry,
        application_id: u64,
    ) -> Result<ApplicationAlbumEntry, DispatchError> {
        cmif::generate_application_album_entry(&self.0, entry, application_id)
    }

    /// Saves an album screenshot file (cmd 2201). \[2.0.0–3.0.2\]
    #[inline]
    pub fn save_album_screenshot_file(
        &self,
        file_id: &AlbumFileId,
        buffer: &[u8],
    ) -> Result<(), SaveScreenShotError> {
        cmif::save_album_screenshot_file(&self.0, file_id, buffer)
    }

    /// Saves an album screenshot file (extended, cmd 2202). \[4.0.0+\]
    #[inline]
    pub fn save_album_screenshot_file_ex(
        &self,
        file_id: &AlbumFileId,
        version: u64,
        makernote_offset: u64,
        makernote_size: u64,
        buffer: &[u8],
    ) -> Result<(), SaveScreenShotError> {
        cmif::save_album_screenshot_file_ex(
            &self.0,
            file_id,
            version,
            makernote_offset,
            makernote_size,
            buffer,
        )
    }

    /// Sets overlay screenshot thumbnail data (cmd 2301). \[2.0.0+\]
    ///
    /// `image` is a 96×54 RGBA8 image buffer.
    #[inline]
    pub fn set_overlay_screenshot_thumbnail_data(
        &self,
        file_id: &AlbumFileId,
        image: &[u8],
    ) -> Result<(), SetOverlayThumbnailError> {
        cmif::set_overlay_thumbnail_data(
            &self.0,
            proto::SET_OVERLAY_SCREENSHOT_THUMBNAIL_DATA,
            file_id,
            image,
        )
    }

    /// Sets overlay movie thumbnail data (cmd 2302). \[4.0.0+\]
    ///
    /// `image` is a 96×54 RGBA8 image buffer.
    #[inline]
    pub fn set_overlay_movie_thumbnail_data(
        &self,
        file_id: &AlbumFileId,
        image: &[u8],
    ) -> Result<(), SetOverlayThumbnailError> {
        cmif::set_overlay_thumbnail_data(
            &self.0,
            proto::SET_OVERLAY_MOVIE_THUMBNAIL_DATA,
            file_id,
            image,
        )
    }

    /// Opens an album control session for movie streaming (cmd 60001). \[4.0.0+\]
    ///
    /// Returns a [`CapscControlSession`] with its own session handle.
    pub fn open_control_session(
        &self,
        applet_resource_user_id: u64,
    ) -> Result<CapscControlSession, OpenControlSessionError> {
        let raw_handle = cmif::open_control_session(&self.0, applet_resource_user_id)?;

        // SAFETY: the kernel returned a valid move handle for the new control
        // session; ownership transfers to the new `Session`.
        let handle = OwnedSessionHandle::from_handle_unchecked(
            RawSessionHandle::from_raw_unchecked(raw_handle),
        );
        Ok(CapscControlSession(Session::new(handle, 0)))
    }
}

/// Album control session wrapper (`IAlbumControlSession`).
///
/// Obtained via [`CapscService::open_control_session`]. Owns its own
/// independent session handle for movie read/write stream operations.
#[repr(transparent)]
pub struct CapscControlSession(Session);

impl CapscControlSession {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// Read stream operations on `CapscControlSession`.
impl CapscControlSession {
    /// Opens an album movie read stream (cmd 2001).
    ///
    /// Up to 4 streams can be open simultaneously. Multiple streams can be
    /// open for the same album file.
    #[inline]
    pub fn open_album_movie_read_stream(
        &self,
        file_id: &AlbumFileId,
    ) -> Result<u64, DispatchError> {
        cmif::ctrl_open_album_movie_read_stream(&self.0, file_id)
    }

    /// Closes an album movie stream (cmd 2002).
    #[inline]
    pub fn close_album_movie_stream(&self, stream: u64) -> Result<(), DispatchError> {
        cmif::ctrl_close_album_movie_stream(&self.0, stream)
    }

    /// Gets the size of an album movie stream (cmd 2003).
    ///
    /// Returns the size of the actual MP4, without the trailing JPEG.
    #[inline]
    pub fn get_album_movie_stream_size(&self, stream: u64) -> Result<u64, DispatchError> {
        cmif::ctrl_get_album_movie_stream_size(&self.0, stream)
    }

    /// Reads movie data from a read stream (cmd 2004).
    ///
    /// `offset` and buffer size must be aligned to 0x40000 bytes.
    /// Returns the actual number of bytes read.
    #[inline]
    pub fn read_movie_data(
        &self,
        stream: u64,
        offset: u64,
        buffer: &mut [u8],
    ) -> Result<u64, ReadStreamDataError> {
        cmif::ctrl_read_movie_data(&self.0, stream, offset, buffer)
    }

    /// Gets the broken reason for a read stream (cmd 2005).
    ///
    /// Returns `Ok(())` if the stream is not broken.
    #[inline]
    pub fn get_read_stream_broken_reason(&self, stream: u64) -> Result<(), DispatchError> {
        cmif::ctrl_get_read_stream_broken_reason(&self.0, stream)
    }

    /// Gets the image data size for a read stream (cmd 2006).
    #[inline]
    pub fn get_read_stream_image_data_size(&self, stream: u64) -> Result<u64, DispatchError> {
        cmif::ctrl_get_read_stream_image_data_size(&self.0, stream)
    }

    /// Reads image data from a read stream (cmd 2007).
    ///
    /// Returns the actual number of bytes read.
    #[inline]
    pub fn read_image_data(
        &self,
        stream: u64,
        offset: u64,
        buffer: &mut [u8],
    ) -> Result<u64, ReadStreamDataError> {
        cmif::ctrl_read_image_data(&self.0, stream, offset, buffer)
    }

    /// Reads the file attribute from a read stream (cmd 2008).
    #[inline]
    pub fn read_file_attribute(&self, stream: u64) -> Result<ScreenShotAttribute, DispatchError> {
        cmif::ctrl_read_file_attribute(&self.0, stream)
    }
}

/// Write stream operations on `CapscControlSession`.
impl CapscControlSession {
    /// Opens an album movie write stream (cmd 2401).
    ///
    /// Up to 2 streams can be open simultaneously.
    #[inline]
    pub fn open_album_movie_write_stream(
        &self,
        file_id: &AlbumFileId,
    ) -> Result<u64, DispatchError> {
        cmif::ctrl_open_album_movie_write_stream(&self.0, file_id)
    }

    /// Finishes a write stream (cmd 2402).
    #[inline]
    pub fn finish_album_movie_write_stream(&self, stream: u64) -> Result<(), DispatchError> {
        cmif::ctrl_finish_write_stream(&self.0, stream)
    }

    /// Commits a finished write stream (cmd 2403).
    #[inline]
    pub fn commit_album_movie_write_stream(&self, stream: u64) -> Result<(), DispatchError> {
        cmif::ctrl_commit_write_stream(&self.0, stream)
    }

    /// Discards a write stream in any state (cmd 2404).
    #[inline]
    pub fn discard_album_movie_write_stream(&self, stream: u64) -> Result<(), DispatchError> {
        cmif::ctrl_discard_write_stream(&self.0, stream)
    }

    /// Discards a write stream without deleting the temp file (cmd 2405).
    #[inline]
    pub fn discard_album_movie_write_stream_no_delete(
        &self,
        stream: u64,
    ) -> Result<(), DispatchError> {
        cmif::ctrl_discard_write_stream_no_delete(&self.0, stream)
    }

    /// Commits a finished write stream, returning the album entry (cmd 2406).
    #[inline]
    pub fn commit_album_movie_write_stream_ex(
        &self,
        stream: u64,
    ) -> Result<AlbumEntry, DispatchError> {
        cmif::ctrl_commit_write_stream_ex(&self.0, stream)
    }

    /// Starts the data section of a write stream (cmd 2411).
    #[inline]
    pub fn start_album_movie_write_stream_data_section(
        &self,
        stream: u64,
    ) -> Result<(), DispatchError> {
        cmif::ctrl_start_write_stream_data_section(&self.0, stream)
    }

    /// Ends the data section of a write stream (cmd 2412).
    #[inline]
    pub fn end_album_movie_write_stream_data_section(
        &self,
        stream: u64,
    ) -> Result<(), DispatchError> {
        cmif::ctrl_end_write_stream_data_section(&self.0, stream)
    }

    /// Starts the meta section of a write stream (cmd 2413).
    #[inline]
    pub fn start_album_movie_write_stream_meta_section(
        &self,
        stream: u64,
    ) -> Result<(), DispatchError> {
        cmif::ctrl_start_write_stream_meta_section(&self.0, stream)
    }

    /// Ends the meta section of a write stream (cmd 2414).
    #[inline]
    pub fn end_album_movie_write_stream_meta_section(
        &self,
        stream: u64,
    ) -> Result<(), DispatchError> {
        cmif::ctrl_end_write_stream_meta_section(&self.0, stream)
    }

    /// Reads data from a write stream (cmd 2421).
    ///
    /// `offset` and buffer size must be aligned to 0x40000 bytes.
    /// Returns the actual number of bytes read.
    #[inline]
    pub fn read_data_from_album_movie_write_stream(
        &self,
        stream: u64,
        offset: u64,
        buffer: &mut [u8],
    ) -> Result<u64, ReadStreamDataError> {
        cmif::ctrl_read_data_from_write_stream(&self.0, stream, offset, buffer)
    }

    /// Writes data to a write stream (cmd 2422).
    #[inline]
    pub fn write_data_to_album_movie_write_stream(
        &self,
        stream: u64,
        offset: u64,
        buffer: &[u8],
    ) -> Result<(), WriteStreamDataError> {
        cmif::ctrl_write_data_to_write_stream(&self.0, stream, offset, buffer)
    }

    /// Writes meta to a write stream (cmd 2424).
    #[inline]
    pub fn write_meta_to_album_movie_write_stream(
        &self,
        stream: u64,
        offset: u64,
        buffer: &[u8],
    ) -> Result<(), WriteStreamDataError> {
        cmif::ctrl_write_meta_to_write_stream(&self.0, stream, offset, buffer)
    }

    /// Gets the broken reason for a write stream (cmd 2431).
    ///
    /// Returns `Ok(())` if the stream is not broken.
    #[inline]
    pub fn get_write_stream_broken_reason(&self, stream: u64) -> Result<(), DispatchError> {
        cmif::ctrl_get_write_stream_broken_reason(&self.0, stream)
    }

    /// Gets the data size of a write stream (cmd 2433).
    #[inline]
    pub fn get_album_movie_write_stream_data_size(
        &self,
        stream: u64,
    ) -> Result<u64, DispatchError> {
        cmif::ctrl_get_write_stream_data_size(&self.0, stream)
    }

    /// Sets the data size of a write stream (cmd 2434).
    ///
    /// Must not be bigger than 2 GiB.
    #[inline]
    pub fn set_album_movie_write_stream_data_size(
        &self,
        stream: u64,
        size: u64,
    ) -> Result<(), DispatchError> {
        cmif::ctrl_set_write_stream_data_size(&self.0, stream, size)
    }
}

/// Connects to the `caps:c` service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<CapscService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(CapscService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get caps:c service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
