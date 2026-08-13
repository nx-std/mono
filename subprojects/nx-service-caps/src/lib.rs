//! Capture (`caps:*`) service implementations.
//!
//! Provides access to the capture services that back the album: browsing and
//! loading album files, capturing and uploading screenshots, and decoding the
//! JPEGs they are stored as.
//!
//! ## Service Variants
//!
//! Six service endpoints are available:
//!
//! - **`caps:a`**: Album accessor. Connected via [`connect_capsa_cmif`].
//! - **`caps:c`**: Album control. Connected via [`connect_capsc_cmif`].
//! - **`caps:dc`**: JPEG decoder. Connected via [`connect_capsdc_cmif`].
//! - **`caps:sc`**: Screenshot control. Connected via [`connect_capssc_cmif`].
//! - **`caps:su`**: Screenshot upload. Connected via [`connect_capssu_cmif`].
//! - **`caps:u`**: Application album. Connected via [`connect_capsu_cmif`].
//!
//! Each variant is an independent endpoint with its own session; connecting to
//! one says nothing about the others. The wire-layout types they share (album
//! entries, file IDs, screenshot attributes, decode options) are declared once
//! at the crate root and re-exported here.
//!
//! ## Hosversion variants
//!
//! This crate is hosversion-unaware: callers choose which methods to call based
//! on the target firmware version. Commands whose wire format differs across
//! versions are exposed as paired `_legacy` (older) and non-suffixed (newer)
//! methods; commands that only exist on newer firmware are exposed
//! unconditionally.
//!
//! ## Naming
//!
//! Where two variants would otherwise export the same name (the connect
//! function, the service name constant, an error type), the name carries the
//! variant it belongs to. Names unique to one variant keep their plain form.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

mod album;
mod caps_a;
mod caps_c;
mod caps_dc;
mod caps_sc;
mod caps_su;
mod caps_u;
mod dispatch;
mod screenshot;
mod user;

pub use self::{
    album::{
        AlbumCache,
        AlbumContentsUsage,
        AlbumContentsUsageFlag,
        AlbumEntry,
        AlbumFileContents,
        AlbumFileContentsFlag,
        AlbumFileDateTime,
        AlbumFileId,
        AlbumReportOption,
        AlbumStorage,
        AlbumUsage2,
        AlbumUsage3,
        AlbumUsage16,
        ApplicationAlbumEntry,
        ApplicationAlbumFileEntry,
        ContentType,
    },
    caps_a::{
        CAPSA_SERVICE_NAME,
        CapsaAccessor,
        CapsaGetAlbumFileListError,
        CapsaOpenAccessorSessionError,
        CapsaReadStreamDataError,
        CapsaService,
        ConnectCapsaCmifError,
        GetAlbumUsage16Error,
        GetMinMaxAppletIdError,
        GetOverlayThumbnailError,
        LoadAlbumFileError,
        LoadScreenShotError,
        MinMaxAppletIdResult,
        OverlayThumbnailResult,
        ScreenShotDimensions,
        ScreenShotImageEx0Result,
        connect_capsa_cmif,
    },
    caps_c::{
        CAPSC_SERVICE_NAME,
        CapsApplicationId,
        CapscControlSession,
        CapscReadStreamDataError,
        CapscService,
        ConnectCapscCmifError,
        OpenControlSessionError,
        SaveScreenShotError,
        SetOverlayThumbnailError,
        WriteStreamDataError,
        connect_capsc_cmif,
    },
    caps_dc::{
        CAPSDC_SERVICE_NAME,
        CapsdcService,
        ConnectCapsdcCmifError,
        DecodeJpegError,
        ShrinkJpegError,
        ShrinkJpegExError,
        connect_capsdc_cmif,
    },
    caps_sc::{
        CAPSSC_SERVICE_NAME,
        CapsscService,
        CaptureJpegError,
        CaptureRawImageError,
        CloseReadStreamError,
        ConnectCapsscCmifError,
        JPEG_BUFFER_SIZE,
        OpenReadStreamError,
        ReadStreamError,
        ReadStreamInfo,
        connect_capssc_cmif,
    },
    caps_su::{
        CAPSSU_SERVICE_NAME,
        CapssuService,
        ConnectCapssuCmifError,
        SaveScreenShotEx0Error,
        SaveScreenShotEx1Error,
        SaveScreenShotEx2Error,
        SetShimVersionError,
        connect_capssu_cmif,
    },
    caps_u::{
        CAPSU_SERVICE_NAME,
        CapsuAccessor,
        CapsuGetAlbumFileListError,
        CapsuOpenAccessorSessionError,
        CapsuService,
        ConnectCapsuCmifError,
        LoadScreenShotImageError,
        ReadMovieDataError,
        connect_capsu_cmif,
    },
    screenshot::{
        AlbumImageOrientation,
        ApplicationData,
        LoadAlbumScreenShotImageOutput,
        LoadAlbumScreenShotImageOutputForApplication,
        ScreenShotAttribute,
        ScreenShotAttributeForApplication,
        ScreenShotDecodeOption,
        ScreenShotDecoderFlag,
    },
    user::{
        AccountUid,
        USER_LIST_SIZE,
        UserIdList,
    },
};
