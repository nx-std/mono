//! Album accessor (`caps:a`) service protocol constants.

use nx_sf::ServiceName;

/// Service name for the album accessor service (`caps:a`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("caps:a");

// IAlbumAccessorService commands

/// Gets the number of album files in a storage.
pub const GET_ALBUM_FILE_COUNT: u32 = 0;

/// Gets a listing of album entries.
pub const GET_ALBUM_FILE_LIST: u32 = 1;

/// Loads an album file into a buffer.
pub const LOAD_ALBUM_FILE: u32 = 2;

/// Deletes an album file.
pub const DELETE_ALBUM_FILE: u32 = 3;

/// Copies an album file to a different storage.
pub const STORAGE_COPY_ALBUM_FILE: u32 = 4;

/// Checks whether a storage is mounted.
pub const IS_ALBUM_MOUNTED: u32 = 5;

/// Gets album usage statistics (2-slot).
pub const GET_ALBUM_USAGE: u32 = 6;

/// Gets the size of an album file.
pub const GET_ALBUM_FILE_SIZE: u32 = 7;

/// Loads the thumbnail for an album file.
pub const LOAD_ALBUM_FILE_THUMBNAIL: u32 = 8;

/// Loads a screenshot image (2.0.0+).
pub const LOAD_ALBUM_SCREEN_SHOT_IMAGE: u32 = 9;

/// Loads a screenshot thumbnail image (2.0.0+).
pub const LOAD_ALBUM_SCREEN_SHOT_THUMBNAIL_IMAGE: u32 = 10;

/// Gets an AlbumEntry from an ApplicationAlbumEntry (2.0.0+).
pub const GET_ALBUM_ENTRY_FROM_APP_ALBUM_ENTRY: u32 = 11;

/// Loads a screenshot image with decode options (3.0.0+).
pub const LOAD_ALBUM_SCREEN_SHOT_IMAGE_EX: u32 = 12;

/// Loads a screenshot thumbnail with decode options (3.0.0+).
pub const LOAD_ALBUM_SCREEN_SHOT_THUMBNAIL_IMAGE_EX: u32 = 13;

/// Loads a screenshot image with decode options and returns attributes (3.0.0+).
pub const LOAD_ALBUM_SCREEN_SHOT_IMAGE_EX0: u32 = 14;

/// Gets album usage statistics (3-slot, 4.0.0+).
pub const GET_ALBUM_USAGE3: u32 = 15;

/// Gets the mount result for a storage (4.0.0+).
pub const GET_ALBUM_MOUNT_RESULT: u32 = 16;

/// Gets album usage statistics (16-slot, 4.0.0+).
pub const GET_ALBUM_USAGE16: u32 = 17;

/// Gets the min/max applet ID range (6.0.0+).
pub const GET_MIN_MAX_APPLET_ID: u32 = 18;

/// Gets the number of album files filtered by type (5.0.0+).
pub const GET_ALBUM_FILE_COUNT_EX0: u32 = 100;

/// Gets a listing of album entries filtered by type (5.0.0+).
pub const GET_ALBUM_FILE_LIST_EX0: u32 = 101;

/// Gets the last overlay screenshot thumbnail.
pub const GET_LAST_OVERLAY_SCREENSHOT_THUMBNAIL: u32 = 301;

/// Gets the last overlay movie thumbnail (4.0.0+).
pub const GET_LAST_OVERLAY_MOVIE_THUMBNAIL: u32 = 302;

/// Gets the auto-saving storage.
pub const GET_AUTO_SAVING_STORAGE: u32 = 401;

/// Gets the required storage space to copy all files between storages.
pub const GET_REQUIRED_STORAGE_SPACE_SIZE_TO_COPY_ALL: u32 = 501;

/// Loads a screenshot thumbnail with decode options and returns attributes (3.0.0+).
pub const LOAD_ALBUM_SCREEN_SHOT_THUMBNAIL_IMAGE_EX0: u32 = 1001;

/// Loads a screenshot image with full output struct (4.0.0+).
pub const LOAD_ALBUM_SCREEN_SHOT_IMAGE_EX1: u32 = 1002;

/// Loads a screenshot thumbnail with full output struct (4.0.0+).
pub const LOAD_ALBUM_SCREEN_SHOT_THUMBNAIL_IMAGE_EX1: u32 = 1003;

/// Force-unmounts a storage.
pub const FORCE_ALBUM_UNMOUNTED: u32 = 8001;

/// Resets album mount status for a storage.
pub const RESET_ALBUM_MOUNT_STATUS: u32 = 8002;

/// Refreshes album cache for a storage.
pub const REFRESH_ALBUM_CACHE: u32 = 8011;

/// Gets album cache for a storage.
pub const GET_ALBUM_CACHE: u32 = 8012;

/// Gets album cache by storage and content type (4.0.0+).
pub const GET_ALBUM_CACHE_EX: u32 = 8013;

/// Gets an AlbumEntry from an ApplicationAlbumEntry with ARUID (2.0.0+).
pub const GET_ALBUM_ENTRY_FROM_APP_ALBUM_ENTRY_ARUID: u32 = 8021;

// Accessor session management

/// Opens an IAlbumAccessorSession.
pub const OPEN_ACCESSOR_SESSION: u32 = 60002;

// IAlbumAccessorSession commands

/// Opens an album movie read stream.
pub const OPEN_ALBUM_MOVIE_READ_STREAM: u32 = 2001;

/// Closes an album movie stream.
pub const CLOSE_ALBUM_MOVIE_STREAM: u32 = 2002;

/// Gets the size of an album movie stream.
pub const GET_ALBUM_MOVIE_STREAM_SIZE: u32 = 2003;

/// Reads movie data from a read stream.
pub const READ_MOVIE_DATA_FROM_STREAM: u32 = 2004;

/// Gets the broken reason for a read stream.
pub const GET_ALBUM_MOVIE_READ_STREAM_BROKEN_REASON: u32 = 2005;

/// Gets the image data size of a read stream.
pub const GET_ALBUM_MOVIE_READ_STREAM_IMAGE_DATA_SIZE: u32 = 2006;

/// Reads image data from a read stream.
pub const READ_IMAGE_DATA_FROM_STREAM: u32 = 2007;

/// Reads file attributes from a read stream.
pub const READ_FILE_ATTRIBUTE_FROM_STREAM: u32 = 2008;
