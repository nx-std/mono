//! NCM service protocol constants.

use nx_sf::ServiceName;

/// Service name for `ncm`.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("ncm");

// ---------------------------------------------------------------------------
// IContentManager root commands
// ---------------------------------------------------------------------------

/// Creates a content storage for the given storage ID (cmd 0).
pub const CREATE_CONTENT_STORAGE: u32 = 0;

/// Creates a content meta database for the given storage ID (cmd 1).
pub const CREATE_CONTENT_META_DATABASE: u32 = 1;

/// Verifies a content storage (cmd 2).
pub const VERIFY_CONTENT_STORAGE: u32 = 2;

/// Verifies a content meta database (cmd 3).
pub const VERIFY_CONTENT_META_DATABASE: u32 = 3;

/// Opens a content storage sub-object (cmd 4).
pub const OPEN_CONTENT_STORAGE: u32 = 4;

/// Opens a content meta database sub-object (cmd 5).
pub const OPEN_CONTENT_META_DATABASE: u32 = 5;

/// Closes content storage forcibly (cmd 6, pre-2.0.0).
pub const CLOSE_CONTENT_STORAGE_FORCIBLY: u32 = 6;

/// Closes content meta database forcibly (cmd 7, pre-2.0.0).
pub const CLOSE_CONTENT_META_DATABASE_FORCIBLY: u32 = 7;

/// Cleans up content meta database (cmd 8).
pub const CLEANUP_CONTENT_META_DATABASE: u32 = 8;

/// Activates a content storage (cmd 9, 2.0.0+).
pub const ACTIVATE_CONTENT_STORAGE: u32 = 9;

/// Inactivates a content storage (cmd 10, 2.0.0+).
pub const INACTIVATE_CONTENT_STORAGE: u32 = 10;

/// Activates a content meta database (cmd 11, 2.0.0+).
pub const ACTIVATE_CONTENT_META_DATABASE: u32 = 11;

/// Inactivates a content meta database (cmd 12, 2.0.0+).
pub const INACTIVATE_CONTENT_META_DATABASE: u32 = 12;

/// Invalidates the rights ID cache (cmd 13, 9.0.0+).
pub const INVALIDATE_RIGHTS_ID_CACHE: u32 = 13;

/// Activates FS content storage (cmd 15, 16.0.0+).
pub const ACTIVATE_FS_CONTENT_STORAGE: u32 = 15;

// ---------------------------------------------------------------------------
// IContentStorage commands
// ---------------------------------------------------------------------------

/// Generates a placeholder ID (cmd 0).
pub const CS_GENERATE_PLACEHOLDER_ID: u32 = 0;

/// Creates a placeholder (cmd 1).
pub const CS_CREATE_PLACEHOLDER: u32 = 1;

/// Deletes a placeholder (cmd 2).
pub const CS_DELETE_PLACEHOLDER: u32 = 2;

/// Checks if a placeholder exists (cmd 3).
pub const CS_HAS_PLACEHOLDER: u32 = 3;

/// Writes data to a placeholder (cmd 4).
pub const CS_WRITE_PLACEHOLDER: u32 = 4;

/// Registers a content ID from a placeholder (cmd 5).
pub const CS_REGISTER: u32 = 5;

/// Deletes a content ID (cmd 6).
pub const CS_DELETE: u32 = 6;

/// Checks if a content ID exists (cmd 7).
pub const CS_HAS: u32 = 7;

/// Gets the filesystem path for a content ID (cmd 8).
pub const CS_GET_PATH: u32 = 8;

/// Gets the filesystem path for a placeholder (cmd 9).
pub const CS_GET_PLACEHOLDER_PATH: u32 = 9;

/// Cleans up all placeholders (cmd 10).
pub const CS_CLEANUP_ALL_PLACEHOLDER: u32 = 10;

/// Lists placeholders (cmd 11).
pub const CS_LIST_PLACEHOLDER: u32 = 11;

/// Gets the content count (cmd 12).
pub const CS_GET_CONTENT_COUNT: u32 = 12;

/// Lists content IDs (cmd 13).
pub const CS_LIST_CONTENT_ID: u32 = 13;

/// Gets the size of a content ID (cmd 14).
pub const CS_GET_SIZE_FROM_CONTENT_ID: u32 = 14;

/// Disables forcibly (cmd 15).
pub const CS_DISABLE_FORCIBLY: u32 = 15;

/// Reverts to a placeholder (cmd 16, 2.0.0+).
pub const CS_REVERT_TO_PLACEHOLDER: u32 = 16;

/// Sets the placeholder size (cmd 17, 2.0.0+).
pub const CS_SET_PLACEHOLDER_SIZE: u32 = 17;

/// Reads content ID file data (cmd 18, 2.0.0+).
pub const CS_READ_CONTENT_ID_FILE: u32 = 18;

/// Gets rights ID from a placeholder ID (cmd 19, 2.0.0+).
pub const CS_GET_RIGHTS_ID_FROM_PLACEHOLDER_ID: u32 = 19;

/// Gets rights ID from a content ID (cmd 20, 2.0.0+).
pub const CS_GET_RIGHTS_ID_FROM_CONTENT_ID: u32 = 20;

/// Writes content data for debug (cmd 21, 2.0.0+).
pub const CS_WRITE_CONTENT_FOR_DEBUG: u32 = 21;

/// Gets free space size (cmd 22, 2.0.0+).
pub const CS_GET_FREE_SPACE_SIZE: u32 = 22;

/// Gets total space size (cmd 23, 2.0.0+).
pub const CS_GET_TOTAL_SPACE_SIZE: u32 = 23;

/// Flushes placeholder data (cmd 24, 3.0.0+).
pub const CS_FLUSH_PLACEHOLDER: u32 = 24;

/// Gets size from a placeholder ID (cmd 25, 4.0.0+).
pub const CS_GET_SIZE_FROM_PLACEHOLDER_ID: u32 = 25;

/// Repairs invalid file attributes (cmd 26, 4.0.0+).
pub const CS_REPAIR_INVALID_FILE_ATTRIBUTE: u32 = 26;

/// Gets rights ID from placeholder with cache (cmd 27, 8.0.0+).
pub const CS_GET_RIGHTS_ID_FROM_PLACEHOLDER_ID_WITH_CACHE: u32 = 27;

/// Registers a path for content (cmd 28, 13.0.0+).
pub const CS_REGISTER_PATH: u32 = 28;

/// Clears registered paths (cmd 29, 13.0.0+).
pub const CS_CLEAR_REGISTERED_PATH: u32 = 29;

/// Gets program ID from content ID (cmd 30, 17.0.0+).
pub const CS_GET_PROGRAM_ID: u32 = 30;

// ---------------------------------------------------------------------------
// IContentMetaDatabase commands
// ---------------------------------------------------------------------------

/// Sets content meta (cmd 0).
pub const DB_SET: u32 = 0;

/// Gets content meta (cmd 1).
pub const DB_GET: u32 = 1;

/// Removes content meta (cmd 2).
pub const DB_REMOVE: u32 = 2;

/// Gets content ID by type (cmd 3).
pub const DB_GET_CONTENT_ID_BY_TYPE: u32 = 3;

/// Lists content info (cmd 4).
pub const DB_LIST_CONTENT_INFO: u32 = 4;

/// Lists content meta keys (cmd 5).
pub const DB_LIST: u32 = 5;

/// Gets the latest content meta key (cmd 6).
pub const DB_GET_LATEST_CONTENT_META_KEY: u32 = 6;

/// Lists application content meta keys (cmd 7).
pub const DB_LIST_APPLICATION: u32 = 7;

/// Checks if a content meta key exists (cmd 8).
pub const DB_HAS: u32 = 8;

/// Checks if all content meta keys exist (cmd 9).
pub const DB_HAS_ALL: u32 = 9;

/// Gets the size of a content meta (cmd 10).
pub const DB_GET_SIZE: u32 = 10;

/// Gets the required system version (cmd 11).
pub const DB_GET_REQUIRED_SYSTEM_VERSION: u32 = 11;

/// Gets the patch content meta ID (cmd 12).
pub const DB_GET_PATCH_CONTENT_META_ID: u32 = 12;

/// Disables forcibly (cmd 13).
pub const DB_DISABLE_FORCIBLY: u32 = 13;

/// Looks up orphan content (cmd 14).
pub const DB_LOOKUP_ORPHAN_CONTENT: u32 = 14;

/// Commits changes (cmd 15).
pub const DB_COMMIT: u32 = 15;

/// Checks if content meta has a specific content (cmd 16).
pub const DB_HAS_CONTENT: u32 = 16;

/// Lists content meta info (cmd 17).
pub const DB_LIST_CONTENT_META_INFO: u32 = 17;

/// Gets attributes (cmd 18).
pub const DB_GET_ATTRIBUTES: u32 = 18;

/// Gets required application version (cmd 19, 2.0.0+).
pub const DB_GET_REQUIRED_APPLICATION_VERSION: u32 = 19;

/// Gets content ID by type and ID offset (cmd 20, 5.0.0+).
pub const DB_GET_CONTENT_ID_BY_TYPE_AND_ID_OFFSET: u32 = 20;

/// Gets platform (cmd 26, 17.0.0+).
pub const DB_GET_PLATFORM: u32 = 26;
