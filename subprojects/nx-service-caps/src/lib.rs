//! Common capture-service (`caps:*`) shared types.
//!
//! This crate provides the wire-layout types, enums, and bitflags shared
//! across multiple capture-service crates (`caps:dc`, `caps:sc`, `caps:su`,
//! `caps:a`, `caps:c`, `caps:u`). It has no IPC commands of its own.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

mod types;

pub use self::types::{
    AccountUid,
    AlbumCache,
    AlbumContentsUsage,
    AlbumContentsUsageFlag,
    AlbumEntry,
    AlbumFileContents,
    AlbumFileContentsFlag,
    AlbumFileDateTime,
    AlbumFileId,
    AlbumImageOrientation,
    AlbumReportOption,
    AlbumStorage,
    AlbumUsage2,
    AlbumUsage3,
    AlbumUsage16,
    ApplicationAlbumEntry,
    ApplicationAlbumFileEntry,
    ApplicationData,
    ContentType,
    LoadAlbumScreenShotImageOutput,
    LoadAlbumScreenShotImageOutputForApplication,
    ScreenShotAttribute,
    ScreenShotAttributeForApplication,
    ScreenShotDecodeOption,
    ScreenShotDecoderFlag,
    USER_LIST_SIZE,
    UserIdList,
};
