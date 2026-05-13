//! Album accessor (`caps:a`) service implementation.
//!
//! Provides access to the album accessor service for browsing, loading, and
//! managing album files (screenshots and movies), as well as streaming movie
//! content via an accessor sub-object.
//!
//! ## Architecture
//!
//! The service is non-domain. [`connect_cmif`] obtains the root session.
//! Movie streaming requires an accessor sub-object obtained via
//! [`CapsaService::open_accessor_session`], which returns a
//! [`CapsaAccessor`] with its own independent session handle.
//!
//! ## Divergence from libnx
//!
//! libnx's `capsa.c` keeps two guarded global singletons (`g_capsaSrv` and
//! `g_capsaAccessor`) managed by `NX_GENERATE_SERVICE_GUARD`, enforces
//! hosversion checks at each call site, and lazily opens the accessor
//! session. This crate follows the convention of the other `nx-service-*`
//! crates: connect once via [`connect_cmif`], then call methods directly.
//!
//! Per IC-4, this crate is hosversion-unaware — callers choose which methods
//! to call based on the target firmware version.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_caps::{
    AlbumCache, AlbumEntry, AlbumFileId, AlbumUsage2, AlbumUsage3, AlbumUsage16,
    ApplicationAlbumEntry, LoadAlbumScreenShotImageOutput, ScreenShotAttribute,
    ScreenShotDecodeOption,
};
use nx_service_sm::SmService;
use nx_sf::service::{DispatchError, Session};
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::{
        GetAlbumFileListError, GetAlbumUsage16Error, GetMinMaxAppletIdError,
        GetOverlayThumbnailError, LoadAlbumFileError, LoadScreenShotError,
        OpenAccessorSessionError, ReadStreamDataError,
    },
    proto::SERVICE_NAME,
    types::{
        MinMaxAppletIdResult, OverlayThumbnailResult, ScreenShotDimensions,
        ScreenShotImageEx0Result,
    },
};

/// Album accessor (`caps:a`) root session wrapper.
///
/// Provides commands for browsing, loading, and managing album files.
/// Use [`open_accessor_session`](Self::open_accessor_session) to create a
/// [`CapsaAccessor`] for movie streaming operations.
#[repr(transparent)]
pub struct CapsaService(Session);

impl CapsaService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods for `CapsaService`.
impl CapsaService {
    /// Gets the number of album files in a storage (cmd 0).
    #[inline]
    pub fn get_album_file_count(&self, storage: u8) -> Result<u64, DispatchError> {
        cmif::get_album_file_count(&self.0, storage)
    }

    /// Gets a listing of album entries (cmd 1).
    ///
    /// `entries` is a byte buffer sized for `count * size_of::<AlbumEntry>()`.
    /// Returns the total number of entries written.
    #[inline]
    pub fn get_album_file_list(
        &self,
        storage: u8,
        entries: &mut [u8],
    ) -> Result<u64, GetAlbumFileListError> {
        cmif::get_album_file_list(&self.0, storage, entries)
    }

    /// Loads an album file into a buffer (cmd 2).
    ///
    /// Returns the size of the file.
    #[inline]
    pub fn load_album_file(
        &self,
        file_id: &AlbumFileId,
        filebuf: &mut [u8],
    ) -> Result<u64, LoadAlbumFileError> {
        cmif::load_album_file(&self.0, file_id, filebuf)
    }

    /// Deletes an album file (cmd 3).
    #[inline]
    pub fn delete_album_file(&self, file_id: &AlbumFileId) -> Result<(), DispatchError> {
        cmif::delete_album_file(&self.0, file_id)
    }

    /// Copies an album file to a different storage (cmd 4).
    #[inline]
    pub fn storage_copy_album_file(
        &self,
        file_id: &AlbumFileId,
        dst_storage: u8,
    ) -> Result<(), DispatchError> {
        cmif::storage_copy_album_file(&self.0, file_id, dst_storage)
    }

    /// Checks whether a storage is mounted (cmd 5).
    #[inline]
    pub fn is_album_mounted(&self, storage: u8) -> Result<bool, DispatchError> {
        cmif::is_album_mounted(&self.0, storage)
    }

    /// Gets album usage statistics (cmd 6).
    #[inline]
    pub fn get_album_usage(&self, storage: u8) -> Result<AlbumUsage2, DispatchError> {
        cmif::get_album_usage(&self.0, storage)
    }

    /// Gets the size of an album file (cmd 7).
    #[inline]
    pub fn get_album_file_size(&self, file_id: &AlbumFileId) -> Result<u64, DispatchError> {
        cmif::get_album_file_size(&self.0, file_id)
    }

    /// Loads the thumbnail for an album file (cmd 8).
    ///
    /// Thumbnails are always 320x180 JPEG. Returns the thumbnail size.
    #[inline]
    pub fn load_album_file_thumbnail(
        &self,
        file_id: &AlbumFileId,
        image: &mut [u8],
    ) -> Result<u64, LoadAlbumFileError> {
        cmif::load_album_file_thumbnail(&self.0, file_id, image)
    }

    /// Loads a screenshot image (cmd 9). \[2.0.0+\]
    ///
    /// `image` is an RGBA8 output buffer (at least 1280x720x4 bytes).
    /// `workbuf` is a work buffer (at least the JPEG size within the album file).
    #[inline]
    pub fn load_album_screen_shot_image(
        &self,
        file_id: &AlbumFileId,
        image: &mut [u8],
        workbuf: &mut [u8],
    ) -> Result<ScreenShotDimensions, LoadScreenShotError> {
        let out = cmif::load_album_screen_shot_image(
            &self.0,
            proto::LOAD_ALBUM_SCREEN_SHOT_IMAGE,
            file_id,
            image,
            workbuf,
        )?;
        Ok(ScreenShotDimensions {
            width: out.width,
            height: out.height,
        })
    }

    /// Loads a screenshot thumbnail image (cmd 10). \[2.0.0+\]
    ///
    /// `image` is an RGBA8 output buffer (at least 320x180x4 bytes).
    /// `workbuf` is a work buffer (at least the JPEG size within the album file).
    #[inline]
    pub fn load_album_screen_shot_thumbnail_image(
        &self,
        file_id: &AlbumFileId,
        image: &mut [u8],
        workbuf: &mut [u8],
    ) -> Result<ScreenShotDimensions, LoadScreenShotError> {
        let out = cmif::load_album_screen_shot_image(
            &self.0,
            proto::LOAD_ALBUM_SCREEN_SHOT_THUMBNAIL_IMAGE,
            file_id,
            image,
            workbuf,
        )?;
        Ok(ScreenShotDimensions {
            width: out.width,
            height: out.height,
        })
    }

    /// Gets an AlbumEntry from an ApplicationAlbumEntry and application ID (cmd 11). \[2.0.0+\]
    #[inline]
    pub fn get_album_entry_from_app_album_entry(
        &self,
        application_entry: &ApplicationAlbumEntry,
        application_id: u64,
    ) -> Result<AlbumEntry, DispatchError> {
        cmif::get_album_entry_from_app_album_entry(&self.0, application_entry, application_id)
    }

    /// Loads a screenshot image with decode options (cmd 12). \[3.0.0+\]
    #[inline]
    pub fn load_album_screen_shot_image_ex(
        &self,
        file_id: &AlbumFileId,
        opts: &ScreenShotDecodeOption,
        image: &mut [u8],
        workbuf: &mut [u8],
    ) -> Result<ScreenShotDimensions, LoadScreenShotError> {
        let out = cmif::load_album_screen_shot_image_ex(
            &self.0,
            proto::LOAD_ALBUM_SCREEN_SHOT_IMAGE_EX,
            file_id,
            opts,
            image,
            workbuf,
        )?;
        Ok(ScreenShotDimensions {
            width: out.width,
            height: out.height,
        })
    }

    /// Loads a screenshot thumbnail with decode options (cmd 13). \[3.0.0+\]
    #[inline]
    pub fn load_album_screen_shot_thumbnail_image_ex(
        &self,
        file_id: &AlbumFileId,
        opts: &ScreenShotDecodeOption,
        image: &mut [u8],
        workbuf: &mut [u8],
    ) -> Result<ScreenShotDimensions, LoadScreenShotError> {
        let out = cmif::load_album_screen_shot_image_ex(
            &self.0,
            proto::LOAD_ALBUM_SCREEN_SHOT_THUMBNAIL_IMAGE_EX,
            file_id,
            opts,
            image,
            workbuf,
        )?;
        Ok(ScreenShotDimensions {
            width: out.width,
            height: out.height,
        })
    }

    /// Loads a screenshot image with decode options, returning attributes (cmd 14). \[3.0.0+\]
    #[inline]
    pub fn load_album_screen_shot_image_ex0(
        &self,
        file_id: &AlbumFileId,
        opts: &ScreenShotDecodeOption,
        image: &mut [u8],
        workbuf: &mut [u8],
    ) -> Result<ScreenShotImageEx0Result, LoadScreenShotError> {
        let out = cmif::load_album_screen_shot_image_ex0(
            &self.0,
            proto::LOAD_ALBUM_SCREEN_SHOT_IMAGE_EX0,
            file_id,
            opts,
            image,
            workbuf,
        )?;
        Ok(ScreenShotImageEx0Result {
            attr: out.attr,
            width: out.width,
            height: out.height,
        })
    }

    /// Gets album usage statistics, 3-slot (cmd 15). \[4.0.0+\]
    #[inline]
    pub fn get_album_usage3(&self, storage: u8) -> Result<AlbumUsage3, DispatchError> {
        cmif::get_album_usage3(&self.0, storage)
    }

    /// Gets the mount result for a storage (cmd 16). \[4.0.0+\]
    #[inline]
    pub fn get_album_mount_result(&self, storage: u8) -> Result<(), DispatchError> {
        cmif::get_album_mount_result(&self.0, storage)
    }

    /// Gets album usage statistics, 16-slot (cmd 17). \[4.0.0+\]
    #[inline]
    pub fn get_album_usage16(
        &self,
        storage: u8,
        flags: u8,
        out: &mut AlbumUsage16,
    ) -> Result<(), GetAlbumUsage16Error> {
        cmif::get_album_usage16(&self.0, storage, flags, out)
    }

    /// Gets the min/max applet ID range (cmd 18). \[6.0.0+\]
    #[inline]
    pub fn get_min_max_applet_id(&self) -> Result<MinMaxAppletIdResult, GetMinMaxAppletIdError> {
        let mut app_ids = [0u64; 2];
        let out = cmif::get_min_max_applet_id(&self.0, &mut app_ids)?;
        Ok(MinMaxAppletIdResult {
            success: out.success != 0,
            min: app_ids[0],
            max: app_ids[1],
        })
    }

    /// Gets album file count filtered by type (cmd 100). \[5.0.0+\]
    #[inline]
    pub fn get_album_file_count_ex0(&self, storage: u8, flags: u8) -> Result<u64, DispatchError> {
        cmif::get_album_file_count_ex0(&self.0, storage, flags)
    }

    /// Gets album file list filtered by type (cmd 101). \[5.0.0+\]
    ///
    /// `entries` is a byte buffer sized for `count * size_of::<AlbumEntry>()`.
    /// Returns the total number of entries written.
    #[inline]
    pub fn get_album_file_list_ex0(
        &self,
        storage: u8,
        flags: u8,
        entries: &mut [u8],
    ) -> Result<u64, GetAlbumFileListError> {
        cmif::get_album_file_list_ex0(&self.0, storage, flags, entries)
    }

    /// Gets the last overlay screenshot thumbnail (cmd 301).
    ///
    /// `image` should be large enough for RGBA8 96x54. Returns the file ID
    /// and thumbnail size (always 0x5100).
    #[inline]
    pub fn get_last_overlay_screenshot_thumbnail(
        &self,
        image: &mut [u8],
    ) -> Result<OverlayThumbnailResult, GetOverlayThumbnailError> {
        let out = cmif::get_last_overlay_thumbnail(
            &self.0,
            proto::GET_LAST_OVERLAY_SCREENSHOT_THUMBNAIL,
            image,
        )?;
        Ok(OverlayThumbnailResult {
            file_id: out.file_id,
            size: out.size,
        })
    }

    /// Gets the last overlay movie thumbnail (cmd 302). \[4.0.0+\]
    ///
    /// `image` should be large enough for RGBA8 96x54. Returns the file ID
    /// and thumbnail size (always 0x5100).
    #[inline]
    pub fn get_last_overlay_movie_thumbnail(
        &self,
        image: &mut [u8],
    ) -> Result<OverlayThumbnailResult, GetOverlayThumbnailError> {
        let out = cmif::get_last_overlay_thumbnail(
            &self.0,
            proto::GET_LAST_OVERLAY_MOVIE_THUMBNAIL,
            image,
        )?;
        Ok(OverlayThumbnailResult {
            file_id: out.file_id,
            size: out.size,
        })
    }

    /// Gets the auto-saving storage (cmd 401).
    #[inline]
    pub fn get_auto_saving_storage(&self) -> Result<u8, DispatchError> {
        cmif::get_auto_saving_storage(&self.0)
    }

    /// Gets the required storage space to copy all files between storages (cmd 501).
    #[inline]
    pub fn get_required_storage_space_size_to_copy_all(
        &self,
        dst_storage: u8,
        src_storage: u8,
    ) -> Result<u64, DispatchError> {
        cmif::get_required_storage_space_size_to_copy_all(&self.0, dst_storage, src_storage)
    }

    /// Loads a screenshot thumbnail with decode options, returning attributes (cmd 1001). \[3.0.0+\]
    #[inline]
    pub fn load_album_screen_shot_thumbnail_image_ex0(
        &self,
        file_id: &AlbumFileId,
        opts: &ScreenShotDecodeOption,
        image: &mut [u8],
        workbuf: &mut [u8],
    ) -> Result<ScreenShotImageEx0Result, LoadScreenShotError> {
        let out = cmif::load_album_screen_shot_image_ex0(
            &self.0,
            proto::LOAD_ALBUM_SCREEN_SHOT_THUMBNAIL_IMAGE_EX0,
            file_id,
            opts,
            image,
            workbuf,
        )?;
        Ok(ScreenShotImageEx0Result {
            attr: out.attr,
            width: out.width,
            height: out.height,
        })
    }

    /// Loads a screenshot image with full output struct (cmd 1002). \[4.0.0+\]
    #[inline]
    pub fn load_album_screen_shot_image_ex1(
        &self,
        file_id: &AlbumFileId,
        opts: &ScreenShotDecodeOption,
        out: &mut LoadAlbumScreenShotImageOutput,
        image: &mut [u8],
        workbuf: &mut [u8],
    ) -> Result<(), LoadScreenShotError> {
        cmif::load_album_screen_shot_image_ex1(
            &self.0,
            proto::LOAD_ALBUM_SCREEN_SHOT_IMAGE_EX1,
            file_id,
            opts,
            out,
            image,
            workbuf,
        )
    }

    /// Loads a screenshot thumbnail with full output struct (cmd 1003). \[4.0.0+\]
    #[inline]
    pub fn load_album_screen_shot_thumbnail_image_ex1(
        &self,
        file_id: &AlbumFileId,
        opts: &ScreenShotDecodeOption,
        out: &mut LoadAlbumScreenShotImageOutput,
        image: &mut [u8],
        workbuf: &mut [u8],
    ) -> Result<(), LoadScreenShotError> {
        cmif::load_album_screen_shot_image_ex1(
            &self.0,
            proto::LOAD_ALBUM_SCREEN_SHOT_THUMBNAIL_IMAGE_EX1,
            file_id,
            opts,
            out,
            image,
            workbuf,
        )
    }

    /// Force-unmounts a storage (cmd 8001).
    #[inline]
    pub fn force_album_unmounted(&self, storage: u8) -> Result<(), DispatchError> {
        cmif::force_album_unmounted(&self.0, storage)
    }

    /// Resets album mount status for a storage (cmd 8002).
    #[inline]
    pub fn reset_album_mount_status(&self, storage: u8) -> Result<(), DispatchError> {
        cmif::reset_album_mount_status(&self.0, storage)
    }

    /// Refreshes album cache for a storage (cmd 8011).
    #[inline]
    pub fn refresh_album_cache(&self, storage: u8) -> Result<(), DispatchError> {
        cmif::refresh_album_cache(&self.0, storage)
    }

    /// Gets album cache for a storage (cmd 8012).
    ///
    /// Stubbed on 4.0.0+; use [`get_album_cache_ex`](Self::get_album_cache_ex) instead.
    #[inline]
    pub fn get_album_cache(&self, storage: u8) -> Result<AlbumCache, DispatchError> {
        cmif::get_album_cache(&self.0, storage)
    }

    /// Gets album cache by storage and content type (cmd 8013). \[4.0.0+\]
    #[inline]
    pub fn get_album_cache_ex(
        &self,
        storage: u8,
        contents: u8,
    ) -> Result<AlbumCache, DispatchError> {
        cmif::get_album_cache_ex(&self.0, storage, contents)
    }

    /// Gets an AlbumEntry from an ApplicationAlbumEntry with ARUID (cmd 8021). \[2.0.0+\]
    #[inline]
    pub fn get_album_entry_from_app_album_entry_aruid(
        &self,
        application_entry: &ApplicationAlbumEntry,
        aruid: u64,
    ) -> Result<AlbumEntry, DispatchError> {
        cmif::get_album_entry_from_app_album_entry_aruid(&self.0, application_entry, aruid)
    }

    /// Opens an accessor session for movie streaming.
    ///
    /// Returns a [`CapsaAccessor`] with its own session handle. The accessor
    /// must be closed separately from the root service (via `Drop`).
    pub fn open_accessor_session(
        &self,
        aruid: u64,
    ) -> Result<CapsaAccessor, OpenAccessorSessionError> {
        let raw_handle = cmif::open_accessor_session(&self.0, aruid)?;

        // SAFETY: the kernel returned a valid move handle for the new accessor
        // session; ownership transfers to the new `Session`.
        let handle = unsafe { SessionHandle::from_raw(raw_handle) };
        Ok(CapsaAccessor(Session::from_handle(handle, 0)))
    }
}

/// Album accessor session wrapper (`IAlbumAccessorSession`).
///
/// Obtained via [`CapsaService::open_accessor_session`]. Owns its own
/// independent session handle for movie stream operations.
#[repr(transparent)]
pub struct CapsaAccessor(Session);

impl CapsaAccessor {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods for `CapsaAccessor`.
impl CapsaAccessor {
    /// Opens an album movie read stream (cmd 2001).
    ///
    /// Up to 4 streams can be open at the same time. Multiple streams can
    /// be open for the same album file.
    ///
    /// Returns the stream handle.
    #[inline]
    pub fn open_album_movie_read_stream(
        &self,
        file_id: &AlbumFileId,
    ) -> Result<u64, DispatchError> {
        cmif::open_album_movie_read_stream(&self.0, file_id)
    }

    /// Closes an album movie stream (cmd 2002).
    #[inline]
    pub fn close_album_movie_stream(&self, stream: u64) -> Result<(), DispatchError> {
        cmif::close_album_movie_stream(&self.0, stream)
    }

    /// Gets the size of an album movie stream (cmd 2003).
    ///
    /// Returns the size of the actual MP4, without the trailing JPEG.
    #[inline]
    pub fn get_album_movie_stream_size(&self, stream: u64) -> Result<u64, DispatchError> {
        cmif::get_album_movie_stream_size(&self.0, stream)
    }

    /// Reads movie data from a read stream (cmd 2004).
    ///
    /// `offset` and the buffer size must be aligned to 0x40000 bytes.
    /// Returns the actual number of bytes read.
    #[inline]
    pub fn read_movie_data_from_stream(
        &self,
        stream: u64,
        offset: i64,
        buffer: &mut [u8],
    ) -> Result<u64, ReadStreamDataError> {
        cmif::read_movie_data_from_stream(&self.0, stream, offset, buffer)
    }

    /// Gets the broken reason for a read stream (cmd 2005).
    ///
    /// Returns `Ok(())` if the stream is not broken.
    #[inline]
    pub fn get_album_movie_read_stream_broken_reason(
        &self,
        stream: u64,
    ) -> Result<(), DispatchError> {
        cmif::get_album_movie_read_stream_broken_reason(&self.0, stream)
    }

    /// Gets the image data size of a read stream (cmd 2006).
    #[inline]
    pub fn get_album_movie_read_stream_image_data_size(
        &self,
        stream: u64,
    ) -> Result<u64, DispatchError> {
        cmif::get_album_movie_read_stream_image_data_size(&self.0, stream)
    }

    /// Reads image data from a read stream (cmd 2007).
    ///
    /// Returns the actual number of bytes read.
    #[inline]
    pub fn read_image_data_from_stream(
        &self,
        stream: u64,
        offset: i64,
        buffer: &mut [u8],
    ) -> Result<u64, ReadStreamDataError> {
        cmif::read_image_data_from_stream(&self.0, stream, offset, buffer)
    }

    /// Reads file attributes from a read stream (cmd 2008).
    #[inline]
    pub fn read_file_attribute_from_stream(
        &self,
        stream: u64,
    ) -> Result<ScreenShotAttribute, DispatchError> {
        cmif::read_file_attribute_from_stream(&self.0, stream)
    }
}

/// Connects to the `caps:a` service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<CapsaService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(CapsaService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get caps:a service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
