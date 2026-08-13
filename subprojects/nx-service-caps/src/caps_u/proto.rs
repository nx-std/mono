//! Application album (`caps:u`) service protocol constants.

use nx_sf::ServiceName;

/// Service name for the application album service (`caps:u`).
pub const CAPSU_SERVICE_NAME: ServiceName = ServiceName::new_truncate("caps:u");

// IApplicationAlbumInterface commands

/// Sets the shim library version. \[7.0.0+\]
pub const SET_SHIM_LIBRARY_VERSION: u32 = 32;

/// Gets album file list by timestamp (pre-6.0.0).
pub const GET_ALBUM_FILE_LIST_DEPRECATED0: u32 = 102;

/// Deletes an album file by ARUID.
pub const DELETE_ALBUM_FILE: u32 = 103;

/// Gets an album file size by ARUID.
pub const GET_ALBUM_FILE_SIZE: u32 = 104;

/// Loads an album screenshot image by ARUID.
pub const LOAD_ALBUM_SCREENSHOT_IMAGE: u32 = 110;

/// Loads an album screenshot thumbnail image by ARUID.
pub const LOAD_ALBUM_SCREENSHOT_THUMBNAIL_IMAGE: u32 = 120;

/// Prechecks to create contents by ARUID.
pub const PRECHECK_TO_CREATE_CONTENTS: u32 = 130;

/// Gets album file list (datetime-based, no UID). \[6.0.0+\]
pub const GET_ALBUM_FILE_LIST_AAE_ARUID: u32 = 140;

/// Gets album file list (datetime-based, with UID). \[6.0.0+\]
pub const GET_ALBUM_FILE_LIST_AAE_UID_ARUID: u32 = 141;

/// Gets album file list (ApplicationAlbumEntry, no UID). \[7.0.0+\]
pub const GET_ALBUM_FILE_LIST3: u32 = 142;

/// Gets album file list (ApplicationAlbumEntry, with UID). \[7.0.0+\]
pub const GET_ALBUM_FILE_LIST4: u32 = 143;

/// Opens an accessor session for the application.
pub const OPEN_ACCESSOR_SESSION: u32 = 60002;

// IAlbumAccessorApplicationSession commands

/// Opens an album movie read stream.
pub const OPEN_ALBUM_MOVIE_READ_STREAM: u32 = 2001;

/// Closes an album movie read stream.
pub const CLOSE_ALBUM_MOVIE_READ_STREAM: u32 = 2002;

/// Gets the movie data size of a read stream.
pub const GET_ALBUM_MOVIE_READ_STREAM_DATA_SIZE: u32 = 2003;

/// Reads movie data from a read stream.
pub const READ_MOVIE_DATA_FROM_STREAM: u32 = 2004;

/// Gets the broken reason for a read stream.
pub const GET_ALBUM_MOVIE_STREAM_BROKEN_REASON: u32 = 2005;
