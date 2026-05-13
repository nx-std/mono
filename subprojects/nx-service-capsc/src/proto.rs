//! Album control (`caps:c`) service protocol constants.

use nx_sf::ServiceName;

/// Service name for the album control service (`caps:c`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("caps:c");

// IAlbumControlService commands

/// Sets the shim library version. \[7.0.0+\]
pub const SET_SHIM_LIBRARY_VERSION: u32 = 33;

/// Notifies that an album storage is available.
pub const NOTIFY_ALBUM_STORAGE_IS_AVAILABLE: u32 = 2001;

/// Notifies that an album storage is unavailable.
pub const NOTIFY_ALBUM_STORAGE_IS_UNAVAILABLE: u32 = 2002;

/// Registers an applet resource user ID. \[2.0.0+\]
pub const REGISTER_APPLET_RESOURCE_USER_ID: u32 = 2011;

/// Unregisters an applet resource user ID. \[2.0.0+\]
pub const UNREGISTER_APPLET_RESOURCE_USER_ID: u32 = 2012;

/// Gets the application ID from an ARUID. \[2.0.0+\]
pub const GET_APPLICATION_ID_FROM_ARUID: u32 = 2013;

/// Checks whether an application ID is registered. \[2.0.0+\]
pub const CHECK_APPLICATION_ID_REGISTERED: u32 = 2014;

/// Generates a current album file ID. \[2.0.0+\]
pub const GENERATE_CURRENT_ALBUM_FILE_ID: u32 = 2101;

/// Generates an application album entry. \[2.0.0+\]
pub const GENERATE_APPLICATION_ALBUM_ENTRY: u32 = 2102;

/// Saves an album screenshot file. \[2.0.0–3.0.2\]
pub const SAVE_ALBUM_SCREENSHOT_FILE: u32 = 2201;

/// Saves an album screenshot file (extended). \[4.0.0+\]
pub const SAVE_ALBUM_SCREENSHOT_FILE_EX: u32 = 2202;

/// Sets overlay screenshot thumbnail data. \[2.0.0+\]
pub const SET_OVERLAY_SCREENSHOT_THUMBNAIL_DATA: u32 = 2301;

/// Sets overlay movie thumbnail data. \[4.0.0+\]
pub const SET_OVERLAY_MOVIE_THUMBNAIL_DATA: u32 = 2302;

/// Opens an album control session. \[4.0.0+\]
pub const OPEN_CONTROL_SESSION: u32 = 60001;

// IAlbumControlSession commands

/// Opens an album movie read stream.
pub const CTRL_OPEN_ALBUM_MOVIE_READ_STREAM: u32 = 2001;

/// Closes an album movie stream.
pub const CTRL_CLOSE_ALBUM_MOVIE_STREAM: u32 = 2002;

/// Gets the size of an album movie stream.
pub const CTRL_GET_ALBUM_MOVIE_STREAM_SIZE: u32 = 2003;

/// Reads movie data from a read stream.
pub const CTRL_READ_MOVIE_DATA_FROM_READ_STREAM: u32 = 2004;

/// Gets the broken reason for a read stream.
pub const CTRL_GET_ALBUM_MOVIE_READ_STREAM_BROKEN_REASON: u32 = 2005;

/// Gets the image data size for a read stream.
pub const CTRL_GET_ALBUM_MOVIE_READ_STREAM_IMAGE_DATA_SIZE: u32 = 2006;

/// Reads image data from a read stream.
pub const CTRL_READ_IMAGE_DATA_FROM_READ_STREAM: u32 = 2007;

/// Reads file attribute from a read stream.
pub const CTRL_READ_FILE_ATTRIBUTE_FROM_READ_STREAM: u32 = 2008;

/// Opens an album movie write stream.
pub const CTRL_OPEN_ALBUM_MOVIE_WRITE_STREAM: u32 = 2401;

/// Finishes an album movie write stream.
pub const CTRL_FINISH_ALBUM_MOVIE_WRITE_STREAM: u32 = 2402;

/// Commits an album movie write stream.
pub const CTRL_COMMIT_ALBUM_MOVIE_WRITE_STREAM: u32 = 2403;

/// Discards an album movie write stream.
pub const CTRL_DISCARD_ALBUM_MOVIE_WRITE_STREAM: u32 = 2404;

/// Discards an album movie write stream without deleting temp file.
pub const CTRL_DISCARD_ALBUM_MOVIE_WRITE_STREAM_NO_DELETE: u32 = 2405;

/// Commits an album movie write stream (extended, returns AlbumEntry).
pub const CTRL_COMMIT_ALBUM_MOVIE_WRITE_STREAM_EX: u32 = 2406;

/// Starts the data section of a write stream.
pub const CTRL_START_WRITE_STREAM_DATA_SECTION: u32 = 2411;

/// Ends the data section of a write stream.
pub const CTRL_END_WRITE_STREAM_DATA_SECTION: u32 = 2412;

/// Starts the meta section of a write stream.
pub const CTRL_START_WRITE_STREAM_META_SECTION: u32 = 2413;

/// Ends the meta section of a write stream.
pub const CTRL_END_WRITE_STREAM_META_SECTION: u32 = 2414;

/// Reads data from a write stream.
pub const CTRL_READ_DATA_FROM_WRITE_STREAM: u32 = 2421;

/// Writes data to a write stream.
pub const CTRL_WRITE_DATA_TO_WRITE_STREAM: u32 = 2422;

/// Writes meta to a write stream.
pub const CTRL_WRITE_META_TO_WRITE_STREAM: u32 = 2424;

/// Gets the broken reason for a write stream.
pub const CTRL_GET_WRITE_STREAM_BROKEN_REASON: u32 = 2431;

/// Gets the data size of a write stream.
pub const CTRL_GET_WRITE_STREAM_DATA_SIZE: u32 = 2433;

/// Sets the data size of a write stream.
pub const CTRL_SET_WRITE_STREAM_DATA_SIZE: u32 = 2434;
