//! Application album (`caps:u`) service implementation.
//!
//! Provides access to the application album service for browsing, loading,
//! and streaming album files (screenshots and movies) associated with the
//! current application.
//!
//! ## Architecture
//!
//! The service is non-domain. [`connect_cmif`] obtains the root session.
//! Movie streaming requires an accessor sub-object obtained via
//! [`CapsuService::open_accessor_session`], which returns a
//! [`CapsuAccessor`] with its own independent session handle.
//!
//! ## Divergence from libnx
//!
//! libnx's `capsu.c` keeps two guarded global singletons (`g_capsuSrv` and
//! `g_capsuAccessor`) managed by `NX_GENERATE_SERVICE_GUARD`, enforces
//! hosversion checks at each call site, and automatically calls
//! `SetShimLibraryVersion` during initialization on 7.0.0+. This crate
//! follows the convention of the other `nx-service-*` crates: connect once
//! via [`connect_cmif`], then call methods directly.
//!
//! Per IC-4, this crate is hosversion-unaware — callers choose which methods
//! to call based on the target firmware version.
//!
//! The convenience wrappers from libnx (`capsuGetAlbumFileListDeprecated1`,
//! `capsuGetAlbumFileListDeprecated2`) that pick commands based on hosversion
//! are not replicated. Instead, each underlying IPC command is exposed
//! directly:
//!
//! - Pre-6.0.0: [`CapsuService::get_album_file_list_deprecated0`] (cmd 102)
//! - 6.0.0+: [`CapsuService::get_album_file_list_aae`] (cmd 140) /
//!   [`CapsuService::get_album_file_list_aae_uid`] (cmd 141)
//! - 7.0.0+: [`CapsuService::get_album_file_list3`] (cmd 142) /
//!   [`CapsuService::get_album_file_list4`] (cmd 143)

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_caps::{
    AccountUid, AlbumFileDateTime, ApplicationAlbumFileEntry,
    LoadAlbumScreenShotImageOutputForApplication, ScreenShotDecodeOption,
};
use nx_service_sm::SmService;
use nx_sf::{
    ipc::Handle as RawSessionHandle,
    service::{BorrowedSessionHandle, DispatchError, OwnedSessionHandle, Session},
};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::{
        GetAlbumFileListError, LoadScreenShotImageError, OpenAccessorSessionError,
        ReadMovieDataError,
    },
    proto::SERVICE_NAME,
};

/// Application album (`caps:u`) root session wrapper.
///
/// Use the album file list methods to enumerate files, load-screenshot methods
/// to decode images, and [`open_accessor_session`](Self::open_accessor_session)
/// to create a [`CapsuAccessor`] for movie streaming.
#[repr(transparent)]
pub struct CapsuService(Session);

impl CapsuService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods for `CapsuService`.
impl CapsuService {
    /// Sets the shim library version. Should be called after connect on 7.0.0+.
    #[inline]
    pub fn set_shim_library_version(
        &self,
        version: u64,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::set_shim_library_version(&self.0, version, applet_resource_user_id)
    }

    /// Gets album file list by timestamp (pre-6.0.0, cmd 102).
    ///
    /// `entries` is a byte buffer sized for `count * size_of::<ApplicationAlbumFileEntry>()`.
    /// Returns the total number of entries written.
    #[inline]
    pub fn get_album_file_list_deprecated0(
        &self,
        content_type: u8,
        start_timestamp: u64,
        end_timestamp: u64,
        applet_resource_user_id: u64,
        entries: &mut [u8],
    ) -> Result<u64, GetAlbumFileListError> {
        cmif::get_album_file_list_deprecated0(
            &self.0,
            content_type,
            start_timestamp,
            end_timestamp,
            applet_resource_user_id,
            entries,
        )
    }

    /// Deletes an album file (cmd 103).
    ///
    /// `content_type` must match `ContentType::ExtraMovie`.
    #[inline]
    pub fn delete_album_file(
        &self,
        content_type: u8,
        entry: &ApplicationAlbumFileEntry,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::delete_album_file(&self.0, content_type, entry, applet_resource_user_id)
    }

    /// Gets the file size for an album file (cmd 104).
    #[inline]
    pub fn get_album_file_size(
        &self,
        entry: &ApplicationAlbumFileEntry,
        applet_resource_user_id: u64,
    ) -> Result<u64, DispatchError> {
        cmif::get_album_file_size(&self.0, entry, applet_resource_user_id)
    }

    /// Loads an album screenshot image (cmd 110).
    ///
    /// `out` receives the image metadata and application data.
    /// `image` is an RGBA8 output buffer (at least 1280x720x4 bytes).
    /// `workbuf` is a work buffer (at least the JPEG size).
    #[inline]
    pub fn load_album_screenshot_image(
        &self,
        entry: &ApplicationAlbumFileEntry,
        option: &ScreenShotDecodeOption,
        applet_resource_user_id: u64,
        out: &mut LoadAlbumScreenShotImageOutputForApplication,
        image: &mut [u8],
        workbuf: &mut [u8],
    ) -> Result<(), LoadScreenShotImageError> {
        cmif::load_album_screenshot_image(
            &self.0,
            proto::LOAD_ALBUM_SCREENSHOT_IMAGE,
            entry,
            option,
            applet_resource_user_id,
            out,
            image,
            workbuf,
        )
    }

    /// Loads an album screenshot thumbnail image (cmd 120).
    ///
    /// `out` receives the image metadata and application data.
    /// `image` is an RGBA8 output buffer (at least 320x180x4 bytes).
    /// `workbuf` is a work buffer (at least the JPEG size).
    #[inline]
    pub fn load_album_screenshot_thumbnail_image(
        &self,
        entry: &ApplicationAlbumFileEntry,
        option: &ScreenShotDecodeOption,
        applet_resource_user_id: u64,
        out: &mut LoadAlbumScreenShotImageOutputForApplication,
        image: &mut [u8],
        workbuf: &mut [u8],
    ) -> Result<(), LoadScreenShotImageError> {
        cmif::load_album_screenshot_image(
            &self.0,
            proto::LOAD_ALBUM_SCREENSHOT_THUMBNAIL_IMAGE,
            entry,
            option,
            applet_resource_user_id,
            out,
            image,
            workbuf,
        )
    }

    /// Prechecks to create contents (cmd 130).
    ///
    /// Official software only uses this with `ContentType::ExtraMovie`.
    #[inline]
    pub fn precheck_to_create_contents(
        &self,
        content_type: u8,
        unk: u64,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::precheck_to_create_contents(&self.0, content_type, unk, applet_resource_user_id)
    }

    /// Gets album file list using `ApplicationAlbumFileEntry` format. \[6.0.0+\]
    ///
    /// `entries` is a byte buffer sized for `count * size_of::<ApplicationAlbumFileEntry>()`.
    /// Returns the total number of entries written.
    #[inline]
    pub fn get_album_file_list_aae(
        &self,
        content_type: u8,
        start_datetime: &AlbumFileDateTime,
        end_datetime: &AlbumFileDateTime,
        applet_resource_user_id: u64,
        entries: &mut [u8],
    ) -> Result<u64, GetAlbumFileListError> {
        cmif::get_album_file_list_aae(
            &self.0,
            proto::GET_ALBUM_FILE_LIST_AAE_ARUID,
            content_type,
            start_datetime,
            end_datetime,
            applet_resource_user_id,
            entries,
        )
    }

    /// Gets album file list filtered by UID using `ApplicationAlbumFileEntry` format. \[6.0.0+\]
    ///
    /// `entries` is a byte buffer sized for `count * size_of::<ApplicationAlbumFileEntry>()`.
    /// Returns the total number of entries written.
    #[inline]
    pub fn get_album_file_list_aae_uid(
        &self,
        content_type: u8,
        start_datetime: &AlbumFileDateTime,
        end_datetime: &AlbumFileDateTime,
        uid: AccountUid,
        applet_resource_user_id: u64,
        entries: &mut [u8],
    ) -> Result<u64, GetAlbumFileListError> {
        cmif::get_album_file_list_aae_uid(
            &self.0,
            proto::GET_ALBUM_FILE_LIST_AAE_UID_ARUID,
            content_type,
            start_datetime,
            end_datetime,
            uid,
            applet_resource_user_id,
            entries,
        )
    }

    /// Gets album file list using `ApplicationAlbumEntry` format. \[7.0.0+\]
    ///
    /// `entries` is a byte buffer sized for `count * size_of::<ApplicationAlbumEntry>()`.
    /// Returns the total number of entries written.
    #[inline]
    pub fn get_album_file_list3(
        &self,
        content_type: u8,
        start_datetime: &AlbumFileDateTime,
        end_datetime: &AlbumFileDateTime,
        applet_resource_user_id: u64,
        entries: &mut [u8],
    ) -> Result<u64, GetAlbumFileListError> {
        cmif::get_album_file_list_aae(
            &self.0,
            proto::GET_ALBUM_FILE_LIST3,
            content_type,
            start_datetime,
            end_datetime,
            applet_resource_user_id,
            entries,
        )
    }

    /// Gets album file list filtered by UID using `ApplicationAlbumEntry` format. \[7.0.0+\]
    ///
    /// `entries` is a byte buffer sized for `count * size_of::<ApplicationAlbumEntry>()`.
    /// Returns the total number of entries written.
    #[inline]
    pub fn get_album_file_list4(
        &self,
        content_type: u8,
        start_datetime: &AlbumFileDateTime,
        end_datetime: &AlbumFileDateTime,
        uid: AccountUid,
        applet_resource_user_id: u64,
        entries: &mut [u8],
    ) -> Result<u64, GetAlbumFileListError> {
        cmif::get_album_file_list_aae_uid(
            &self.0,
            proto::GET_ALBUM_FILE_LIST4,
            content_type,
            start_datetime,
            end_datetime,
            uid,
            applet_resource_user_id,
            entries,
        )
    }

    /// Opens an accessor session for movie streaming.
    ///
    /// Returns a [`CapsuAccessor`] with its own session handle. The accessor
    /// must be closed separately from the root service.
    pub fn open_accessor_session(
        &self,
        entry: &ApplicationAlbumFileEntry,
        applet_resource_user_id: u64,
    ) -> Result<CapsuAccessor, OpenAccessorSessionError> {
        let raw_handle = cmif::open_accessor_session(&self.0, entry, applet_resource_user_id)?;

        // SAFETY: the kernel returned a valid move handle for the new accessor
        // session; ownership transfers to the new `Session`.
        let handle = OwnedSessionHandle::from_handle_unchecked(
            RawSessionHandle::from_raw_unchecked(raw_handle),
        );
        Ok(CapsuAccessor(Session::new(handle, 0)))
    }
}

/// Album accessor session wrapper (`IAlbumAccessorApplicationSession`).
///
/// Obtained via [`CapsuService::open_accessor_session`]. Owns its own
/// independent session handle for movie stream operations.
#[repr(transparent)]
pub struct CapsuAccessor(Session);

impl CapsuAccessor {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods for `CapsuAccessor`.
impl CapsuAccessor {
    /// Opens an album movie read stream (cmd 2001).
    ///
    /// Up to 4 streams can be open at the same time. Multiple streams can
    /// be open for the same album file entry.
    ///
    /// Returns the stream handle.
    #[inline]
    pub fn open_album_movie_read_stream(
        &self,
        entry: &ApplicationAlbumFileEntry,
        applet_resource_user_id: u64,
    ) -> Result<u64, DispatchError> {
        cmif::open_album_movie_read_stream(&self.0, entry, applet_resource_user_id)
    }

    /// Closes an album movie read stream (cmd 2002).
    #[inline]
    pub fn close_album_movie_read_stream(&self, stream: u64) -> Result<(), DispatchError> {
        cmif::close_album_movie_read_stream(&self.0, stream)
    }

    /// Gets the data size of an album movie read stream (cmd 2003).
    ///
    /// Returns the size of the actual MP4, without the trailing JPEG.
    #[inline]
    pub fn get_album_movie_read_stream_data_size(&self, stream: u64) -> Result<u64, DispatchError> {
        cmif::get_album_movie_read_stream_data_size(&self.0, stream)
    }

    /// Reads data from an album movie read stream (cmd 2004).
    ///
    /// `offset` and the buffer size must be aligned to 0x40000 bytes.
    /// Returns the actual number of bytes read.
    #[inline]
    pub fn read_movie_data(
        &self,
        stream: u64,
        offset: i64,
        buffer: &mut [u8],
    ) -> Result<u64, ReadMovieDataError> {
        cmif::read_movie_data(&self.0, stream, offset, buffer)
    }

    /// Gets the broken reason for an album movie read stream (cmd 2005).
    ///
    /// Returns `Ok(())` if the stream is not broken.
    #[inline]
    pub fn get_album_movie_stream_broken_reason(&self, stream: u64) -> Result<(), DispatchError> {
        cmif::get_album_movie_stream_broken_reason(&self.0, stream)
    }
}

/// Connects to the `caps:u` service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<CapsuService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(CapsuService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get caps:u service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
