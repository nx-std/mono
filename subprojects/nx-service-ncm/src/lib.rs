//! Nintendo Content Manager (`ncm`) service implementation.
//!
//! Provides access to the NCM service for managing installed content and
//! content meta databases on the Nintendo Switch.
//!
//! ## Architecture
//!
//! - **Root session** (`ncm`): Non-domain service providing storage and
//!   meta-database management commands.
//!
//! - **IContentStorage** sub-object: Obtained via
//!   [`NcmService::open_content_storage`]. Manages content files and
//!   placeholders for a given storage location.
//!
//! - **IContentMetaDatabase** sub-object: Obtained via
//!   [`NcmService::open_content_meta_database`]. Provides content meta
//!   queries and mutations for a given storage location.
//!
//! ## Divergence from libnx
//!
//! libnx's `ncm.c` keeps a guarded global singleton managed by
//! `NX_GENERATE_SERVICE_GUARD`. This crate follows the convention of the
//! other `nx-service-*` crates: connect once via [`connect_cmif`], then
//! call methods directly.
//!
//! Several commands have hosversion-dependent wire layouts (different field
//! ordering pre/post 16.0.0, different output sizes pre/post 3.0.0). These
//! are exposed as paired `_legacy` / versioned method variants per IC-4.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::{
    ipc::Handle as RawSessionHandle,
    service::{BorrowedSessionHandle, DispatchError, OwnedSessionHandle, Session},
};

mod cmif;
mod dispatch;
mod proto;
pub mod types;

pub use nx_sf::service::DispatchError as IpcDispatchError;

pub use crate::{
    dispatch::OpenSubObjectError,
    proto::SERVICE_NAME,
    types::{
        FS_MAX_PATH, FsContentAttributes, NcmAddOnContentMetaExtendedHeader,
        NcmApplicationContentMetaKey, NcmApplicationMetaExtendedHeader, NcmContentId,
        NcmContentInfo, NcmContentInstallType, NcmContentMetaHeader, NcmContentMetaInfo,
        NcmContentMetaKey, NcmContentMetaPlatform, NcmContentMetaType, NcmContentType,
        NcmDataPatchMetaExtendedHeader, NcmLegacyAddOnContentMetaExtendedHeader,
        NcmPackagedContentInfo, NcmPatchMetaExtendedHeader, NcmPlaceHolderId, NcmProgramLocation,
        NcmRightsId, NcmStorageId, NcmSystemUpdateMetaExtendedHeader,
    },
};

/// NCM root service wrapper.
#[repr(transparent)]
pub struct NcmService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for NcmService {}
unsafe impl Sync for NcmService {}

impl NcmService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    // -----------------------------------------------------------------------
    // Root IContentManager commands
    // -----------------------------------------------------------------------

    /// Creates a content storage (cmd 0).
    #[inline]
    pub fn create_content_storage(&self, storage_id: NcmStorageId) -> Result<(), DispatchError> {
        cmif::root::create_content_storage(&self.0, storage_id)
    }

    /// Creates a content meta database (cmd 1).
    #[inline]
    pub fn create_content_meta_database(
        &self,
        storage_id: NcmStorageId,
    ) -> Result<(), DispatchError> {
        cmif::root::create_content_meta_database(&self.0, storage_id)
    }

    /// Verifies a content storage (cmd 2).
    #[inline]
    pub fn verify_content_storage(&self, storage_id: NcmStorageId) -> Result<(), DispatchError> {
        cmif::root::verify_content_storage(&self.0, storage_id)
    }

    /// Verifies a content meta database (cmd 3).
    #[inline]
    pub fn verify_content_meta_database(
        &self,
        storage_id: NcmStorageId,
    ) -> Result<(), DispatchError> {
        cmif::root::verify_content_meta_database(&self.0, storage_id)
    }

    /// Opens a content storage sub-object (cmd 4).
    #[inline]
    pub fn open_content_storage(
        &self,
        storage_id: NcmStorageId,
    ) -> Result<NcmContentStorage, OpenSubObjectError> {
        let raw = cmif::root::open_content_storage(&self.0, storage_id)?;
        // SAFETY: The server returned a freshly opened content-storage session in this reply,
        // so the `Session` below is its sole owner.
        let handle =
            OwnedSessionHandle::from_handle_unchecked(RawSessionHandle::from_raw_unchecked(raw));
        Ok(NcmContentStorage(Session::new(handle, 0)))
    }

    /// Opens a content meta database sub-object (cmd 5).
    #[inline]
    pub fn open_content_meta_database(
        &self,
        storage_id: NcmStorageId,
    ) -> Result<NcmContentMetaDatabase, OpenSubObjectError> {
        let raw = cmif::root::open_content_meta_database(&self.0, storage_id)?;
        // SAFETY: The server returned a freshly opened meta-database session in this reply,
        // so the `Session` below is its sole owner.
        let handle =
            OwnedSessionHandle::from_handle_unchecked(RawSessionHandle::from_raw_unchecked(raw));
        Ok(NcmContentMetaDatabase(Session::new(handle, 0)))
    }

    /// Closes content storage forcibly (cmd 6, pre-2.0.0).
    #[inline]
    pub fn close_content_storage_forcibly(
        &self,
        storage_id: NcmStorageId,
    ) -> Result<(), DispatchError> {
        cmif::root::close_content_storage_forcibly(&self.0, storage_id)
    }

    /// Closes content meta database forcibly (cmd 7, pre-2.0.0).
    #[inline]
    pub fn close_content_meta_database_forcibly(
        &self,
        storage_id: NcmStorageId,
    ) -> Result<(), DispatchError> {
        cmif::root::close_content_meta_database_forcibly(&self.0, storage_id)
    }

    /// Cleans up content meta database (cmd 8).
    #[inline]
    pub fn cleanup_content_meta_database(
        &self,
        storage_id: NcmStorageId,
    ) -> Result<(), DispatchError> {
        cmif::root::cleanup_content_meta_database(&self.0, storage_id)
    }

    /// Activates a content storage (cmd 9, 2.0.0+).
    #[inline]
    pub fn activate_content_storage(&self, storage_id: NcmStorageId) -> Result<(), DispatchError> {
        cmif::root::activate_content_storage(&self.0, storage_id)
    }

    /// Inactivates a content storage (cmd 10, 2.0.0+).
    #[inline]
    pub fn inactivate_content_storage(
        &self,
        storage_id: NcmStorageId,
    ) -> Result<(), DispatchError> {
        cmif::root::inactivate_content_storage(&self.0, storage_id)
    }

    /// Activates a content meta database (cmd 11, 2.0.0+).
    #[inline]
    pub fn activate_content_meta_database(
        &self,
        storage_id: NcmStorageId,
    ) -> Result<(), DispatchError> {
        cmif::root::activate_content_meta_database(&self.0, storage_id)
    }

    /// Inactivates a content meta database (cmd 12, 2.0.0+).
    #[inline]
    pub fn inactivate_content_meta_database(
        &self,
        storage_id: NcmStorageId,
    ) -> Result<(), DispatchError> {
        cmif::root::inactivate_content_meta_database(&self.0, storage_id)
    }

    /// Invalidates the rights ID cache (cmd 13, 9.0.0+).
    #[inline]
    pub fn invalidate_rights_id_cache(&self) -> Result<(), DispatchError> {
        cmif::root::invalidate_rights_id_cache(&self.0)
    }

    /// Activates FS content storage (cmd 15, 16.0.0+).
    #[inline]
    pub fn activate_fs_content_storage(&self, fs_storage_id: u32) -> Result<(), DispatchError> {
        cmif::root::activate_fs_content_storage(&self.0, fs_storage_id)
    }
}

// ---------------------------------------------------------------------------
// IContentStorage sub-object
// ---------------------------------------------------------------------------

/// Content storage sub-object wrapper.
#[repr(transparent)]
pub struct NcmContentStorage(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for NcmContentStorage {}
unsafe impl Sync for NcmContentStorage {}

impl NcmContentStorage {
    /// Generates a placeholder ID (cmd 0).
    #[inline]
    pub fn generate_placeholder_id(&self) -> Result<NcmPlaceHolderId, DispatchError> {
        cmif::content_storage::generate_placeholder_id(&self.0)
    }

    /// Creates a placeholder, pre-16.0.0 field ordering (cmd 1).
    #[inline]
    pub fn create_placeholder_legacy(
        &self,
        content_id: &NcmContentId,
        placeholder_id: &NcmPlaceHolderId,
        size: i64,
    ) -> Result<(), DispatchError> {
        cmif::content_storage::create_placeholder_legacy(&self.0, content_id, placeholder_id, size)
    }

    /// Creates a placeholder, 16.0.0+ field ordering (cmd 1).
    #[inline]
    pub fn create_placeholder(
        &self,
        content_id: &NcmContentId,
        placeholder_id: &NcmPlaceHolderId,
        size: i64,
    ) -> Result<(), DispatchError> {
        cmif::content_storage::create_placeholder(&self.0, content_id, placeholder_id, size)
    }

    /// Deletes a placeholder (cmd 2).
    #[inline]
    pub fn delete_placeholder(
        &self,
        placeholder_id: &NcmPlaceHolderId,
    ) -> Result<(), DispatchError> {
        cmif::content_storage::delete_placeholder(&self.0, placeholder_id)
    }

    /// Checks if a placeholder exists (cmd 3).
    #[inline]
    pub fn has_placeholder(
        &self,
        placeholder_id: &NcmPlaceHolderId,
    ) -> Result<bool, DispatchError> {
        cmif::content_storage::has_placeholder(&self.0, placeholder_id)
    }

    /// Writes data to a placeholder (cmd 4).
    #[inline]
    pub fn write_placeholder(
        &self,
        placeholder_id: &NcmPlaceHolderId,
        offset: u64,
        data: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::content_storage::write_placeholder(&self.0, placeholder_id, offset, data)
    }

    /// Registers a content ID from a placeholder, pre-16.0.0 field ordering (cmd 5).
    #[inline]
    pub fn register_legacy(
        &self,
        content_id: &NcmContentId,
        placeholder_id: &NcmPlaceHolderId,
    ) -> Result<(), DispatchError> {
        cmif::content_storage::register_legacy(&self.0, content_id, placeholder_id)
    }

    /// Registers a content ID from a placeholder, 16.0.0+ field ordering (cmd 5).
    #[inline]
    pub fn register(
        &self,
        content_id: &NcmContentId,
        placeholder_id: &NcmPlaceHolderId,
    ) -> Result<(), DispatchError> {
        cmif::content_storage::register(&self.0, content_id, placeholder_id)
    }

    /// Deletes a content ID (cmd 6).
    #[inline]
    pub fn delete(&self, content_id: &NcmContentId) -> Result<(), DispatchError> {
        cmif::content_storage::delete(&self.0, content_id)
    }

    /// Checks if a content ID exists (cmd 7).
    #[inline]
    pub fn has(&self, content_id: &NcmContentId) -> Result<bool, DispatchError> {
        cmif::content_storage::has(&self.0, content_id)
    }

    /// Gets the filesystem path for a content ID (cmd 8).
    #[inline]
    pub fn get_path(
        &self,
        content_id: &NcmContentId,
        out_path: &mut [u8; FS_MAX_PATH],
    ) -> Result<(), DispatchError> {
        cmif::content_storage::get_path(&self.0, content_id, out_path)
    }

    /// Gets the filesystem path for a placeholder (cmd 9).
    #[inline]
    pub fn get_placeholder_path(
        &self,
        placeholder_id: &NcmPlaceHolderId,
        out_path: &mut [u8; FS_MAX_PATH],
    ) -> Result<(), DispatchError> {
        cmif::content_storage::get_placeholder_path(&self.0, placeholder_id, out_path)
    }

    /// Cleans up all placeholders (cmd 10).
    #[inline]
    pub fn cleanup_all_placeholder(&self) -> Result<(), DispatchError> {
        cmif::content_storage::cleanup_all_placeholder(&self.0)
    }

    /// Lists placeholders (cmd 11).
    #[inline]
    pub fn list_placeholder(&self, out_ids: &mut [NcmPlaceHolderId]) -> Result<i32, DispatchError> {
        cmif::content_storage::list_placeholder(&self.0, out_ids)
    }

    /// Gets the content count (cmd 12).
    #[inline]
    pub fn get_content_count(&self) -> Result<i32, DispatchError> {
        cmif::content_storage::get_content_count(&self.0)
    }

    /// Lists content IDs (cmd 13).
    #[inline]
    pub fn list_content_id(
        &self,
        out_ids: &mut [NcmContentId],
        start_offset: i32,
    ) -> Result<i32, DispatchError> {
        cmif::content_storage::list_content_id(&self.0, out_ids, start_offset)
    }

    /// Gets the size of a content ID (cmd 14).
    #[inline]
    pub fn get_size_from_content_id(
        &self,
        content_id: &NcmContentId,
    ) -> Result<i64, DispatchError> {
        cmif::content_storage::get_size_from_content_id(&self.0, content_id)
    }

    /// Disables forcibly (cmd 15).
    #[inline]
    pub fn disable_forcibly(&self) -> Result<(), DispatchError> {
        cmif::content_storage::disable_forcibly(&self.0)
    }

    /// Reverts to a placeholder, pre-16.0.0 field ordering (cmd 16, 2.0.0+).
    #[inline]
    pub fn revert_to_placeholder_legacy(
        &self,
        placeholder_id: &NcmPlaceHolderId,
        old_content_id: &NcmContentId,
        new_content_id: &NcmContentId,
    ) -> Result<(), DispatchError> {
        cmif::content_storage::revert_to_placeholder_legacy(
            &self.0,
            placeholder_id,
            old_content_id,
            new_content_id,
        )
    }

    /// Reverts to a placeholder, 16.0.0+ field ordering (cmd 16).
    #[inline]
    pub fn revert_to_placeholder(
        &self,
        placeholder_id: &NcmPlaceHolderId,
        old_content_id: &NcmContentId,
        new_content_id: &NcmContentId,
    ) -> Result<(), DispatchError> {
        cmif::content_storage::revert_to_placeholder(
            &self.0,
            placeholder_id,
            old_content_id,
            new_content_id,
        )
    }

    /// Sets the placeholder size (cmd 17, 2.0.0+).
    #[inline]
    pub fn set_placeholder_size(
        &self,
        placeholder_id: &NcmPlaceHolderId,
        size: i64,
    ) -> Result<(), DispatchError> {
        cmif::content_storage::set_placeholder_size(&self.0, placeholder_id, size)
    }

    /// Reads content ID file data (cmd 18, 2.0.0+).
    #[inline]
    pub fn read_content_id_file(
        &self,
        content_id: &NcmContentId,
        offset: i64,
        out_data: &mut [u8],
    ) -> Result<(), DispatchError> {
        cmif::content_storage::read_content_id_file(&self.0, content_id, offset, out_data)
    }

    /// Gets rights ID from a placeholder ID, pre-3.0.0 (cmd 19, 2.0.0+).
    #[inline]
    pub fn get_rights_id_from_placeholder_id_legacy(
        &self,
        placeholder_id: &NcmPlaceHolderId,
        attr: FsContentAttributes,
    ) -> Result<[u8; 0x10], DispatchError> {
        cmif::content_storage::get_rights_id_from_placeholder_id_legacy(
            &self.0,
            placeholder_id,
            attr,
        )
    }

    /// Gets rights ID from a placeholder ID, 3.0.0+ (cmd 19).
    #[inline]
    pub fn get_rights_id_from_placeholder_id(
        &self,
        placeholder_id: &NcmPlaceHolderId,
        attr: FsContentAttributes,
    ) -> Result<NcmRightsId, DispatchError> {
        cmif::content_storage::get_rights_id_from_placeholder_id(&self.0, placeholder_id, attr)
    }

    /// Gets rights ID from a content ID, pre-3.0.0 (cmd 20, 2.0.0+).
    #[inline]
    pub fn get_rights_id_from_content_id_legacy(
        &self,
        content_id: &NcmContentId,
        attr: FsContentAttributes,
    ) -> Result<[u8; 0x10], DispatchError> {
        cmif::content_storage::get_rights_id_from_content_id_legacy(&self.0, content_id, attr)
    }

    /// Gets rights ID from a content ID, 3.0.0+ (cmd 20).
    #[inline]
    pub fn get_rights_id_from_content_id(
        &self,
        content_id: &NcmContentId,
        attr: FsContentAttributes,
    ) -> Result<NcmRightsId, DispatchError> {
        cmif::content_storage::get_rights_id_from_content_id(&self.0, content_id, attr)
    }

    /// Writes content data for debug (cmd 21, 2.0.0+).
    #[inline]
    pub fn write_content_for_debug(
        &self,
        content_id: &NcmContentId,
        offset: i64,
        data: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::content_storage::write_content_for_debug(&self.0, content_id, offset, data)
    }

    /// Gets free space size (cmd 22, 2.0.0+).
    #[inline]
    pub fn get_free_space_size(&self) -> Result<i64, DispatchError> {
        cmif::content_storage::get_free_space_size(&self.0)
    }

    /// Gets total space size (cmd 23, 2.0.0+).
    #[inline]
    pub fn get_total_space_size(&self) -> Result<i64, DispatchError> {
        cmif::content_storage::get_total_space_size(&self.0)
    }

    /// Flushes placeholder data (cmd 24, 3.0.0+).
    #[inline]
    pub fn flush_placeholder(&self) -> Result<(), DispatchError> {
        cmif::content_storage::flush_placeholder(&self.0)
    }

    /// Gets size from a placeholder ID (cmd 25, 4.0.0+).
    #[inline]
    pub fn get_size_from_placeholder_id(
        &self,
        placeholder_id: &NcmPlaceHolderId,
    ) -> Result<i64, DispatchError> {
        cmif::content_storage::get_size_from_placeholder_id(&self.0, placeholder_id)
    }

    /// Repairs invalid file attributes (cmd 26, 4.0.0+).
    #[inline]
    pub fn repair_invalid_file_attribute(&self) -> Result<(), DispatchError> {
        cmif::content_storage::repair_invalid_file_attribute(&self.0)
    }

    /// Gets rights ID from placeholder with cache, pre-16.0.0 (cmd 27, 8.0.0+).
    #[inline]
    pub fn get_rights_id_from_placeholder_id_with_cache_legacy(
        &self,
        placeholder_id: &NcmPlaceHolderId,
        cache_content_id: &NcmContentId,
    ) -> Result<NcmRightsId, DispatchError> {
        cmif::content_storage::get_rights_id_from_placeholder_id_with_cache_legacy(
            &self.0,
            placeholder_id,
            cache_content_id,
        )
    }

    /// Gets rights ID from placeholder with cache, 16.0.0+ (cmd 27).
    #[inline]
    pub fn get_rights_id_from_placeholder_id_with_cache(
        &self,
        placeholder_id: &NcmPlaceHolderId,
        cache_content_id: &NcmContentId,
        attr: FsContentAttributes,
    ) -> Result<NcmRightsId, DispatchError> {
        cmif::content_storage::get_rights_id_from_placeholder_id_with_cache(
            &self.0,
            placeholder_id,
            cache_content_id,
            attr,
        )
    }

    /// Registers a path for content (cmd 28, 13.0.0+).
    #[inline]
    pub fn register_path(
        &self,
        content_id: &NcmContentId,
        path: &[u8; FS_MAX_PATH],
    ) -> Result<(), DispatchError> {
        cmif::content_storage::register_path(&self.0, content_id, path)
    }

    /// Clears registered paths (cmd 29, 13.0.0+).
    #[inline]
    pub fn clear_registered_path(&self) -> Result<(), DispatchError> {
        cmif::content_storage::clear_registered_path(&self.0)
    }

    /// Gets program ID from content ID (cmd 30, 17.0.0+).
    #[inline]
    pub fn get_program_id(
        &self,
        content_id: &NcmContentId,
        attr: FsContentAttributes,
    ) -> Result<u64, DispatchError> {
        cmif::content_storage::get_program_id(&self.0, content_id, attr)
    }
}

// ---------------------------------------------------------------------------
// IContentMetaDatabase sub-object
// ---------------------------------------------------------------------------

/// Content meta database sub-object wrapper.
#[repr(transparent)]
pub struct NcmContentMetaDatabase(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for NcmContentMetaDatabase {}
unsafe impl Sync for NcmContentMetaDatabase {}

impl NcmContentMetaDatabase {
    /// Sets content meta (cmd 0).
    #[inline]
    pub fn set(&self, key: &NcmContentMetaKey, data: &[u8]) -> Result<(), DispatchError> {
        cmif::content_meta_database::set(&self.0, key, data)
    }

    /// Gets content meta (cmd 1).
    #[inline]
    pub fn get(&self, key: &NcmContentMetaKey, out_data: &mut [u8]) -> Result<u64, DispatchError> {
        cmif::content_meta_database::get(&self.0, key, out_data)
    }

    /// Removes content meta (cmd 2).
    #[inline]
    pub fn remove(&self, key: &NcmContentMetaKey) -> Result<(), DispatchError> {
        cmif::content_meta_database::remove(&self.0, key)
    }

    /// Gets content ID by type (cmd 3).
    #[inline]
    pub fn get_content_id_by_type(
        &self,
        key: &NcmContentMetaKey,
        content_type: NcmContentType,
    ) -> Result<NcmContentId, DispatchError> {
        cmif::content_meta_database::get_content_id_by_type(&self.0, key, content_type)
    }

    /// Lists content info (cmd 4).
    #[inline]
    pub fn list_content_info(
        &self,
        key: &NcmContentMetaKey,
        start_index: i32,
        out_info: &mut [NcmContentInfo],
    ) -> Result<i32, DispatchError> {
        cmif::content_meta_database::list_content_info(&self.0, key, start_index, out_info)
    }

    /// Lists content meta keys (cmd 5).
    ///
    /// Returns `(entries_total, entries_written)`.
    #[inline]
    pub fn list(
        &self,
        meta_type: NcmContentMetaType,
        id: u64,
        id_min: u64,
        id_max: u64,
        install_type: NcmContentInstallType,
        out_keys: &mut [NcmContentMetaKey],
    ) -> Result<(i32, i32), DispatchError> {
        let out = cmif::content_meta_database::list(
            &self.0,
            meta_type,
            id,
            id_min,
            id_max,
            install_type as u8,
            out_keys,
        )?;
        Ok((out.entries_total, out.entries_written))
    }

    /// Gets the latest content meta key (cmd 6).
    #[inline]
    pub fn get_latest_content_meta_key(&self, id: u64) -> Result<NcmContentMetaKey, DispatchError> {
        cmif::content_meta_database::get_latest_content_meta_key(&self.0, id)
    }

    /// Lists application content meta keys (cmd 7).
    ///
    /// Returns `(entries_total, entries_written)`.
    #[inline]
    pub fn list_application(
        &self,
        meta_type: NcmContentMetaType,
        out_keys: &mut [NcmApplicationContentMetaKey],
    ) -> Result<(i32, i32), DispatchError> {
        let out = cmif::content_meta_database::list_application(&self.0, meta_type, out_keys)?;
        Ok((out.entries_total, out.entries_written))
    }

    /// Checks if a content meta key exists (cmd 8).
    #[inline]
    pub fn has(&self, key: &NcmContentMetaKey) -> Result<bool, DispatchError> {
        cmif::content_meta_database::has(&self.0, key)
    }

    /// Checks if all content meta keys exist (cmd 9).
    #[inline]
    pub fn has_all(&self, keys: &[NcmContentMetaKey]) -> Result<bool, DispatchError> {
        cmif::content_meta_database::has_all(&self.0, keys)
    }

    /// Gets the size of a content meta (cmd 10).
    #[inline]
    pub fn get_size(&self, key: &NcmContentMetaKey) -> Result<u64, DispatchError> {
        cmif::content_meta_database::get_size(&self.0, key)
    }

    /// Gets the required system version (cmd 11).
    #[inline]
    pub fn get_required_system_version(
        &self,
        key: &NcmContentMetaKey,
    ) -> Result<u32, DispatchError> {
        cmif::content_meta_database::get_required_system_version(&self.0, key)
    }

    /// Gets the patch content meta ID (cmd 12).
    #[inline]
    pub fn get_patch_content_meta_id(&self, key: &NcmContentMetaKey) -> Result<u64, DispatchError> {
        cmif::content_meta_database::get_patch_content_meta_id(&self.0, key)
    }

    /// Disables forcibly (cmd 13).
    #[inline]
    pub fn disable_forcibly(&self) -> Result<(), DispatchError> {
        cmif::content_meta_database::disable_forcibly(&self.0)
    }

    /// Looks up orphan content (cmd 14).
    ///
    /// For each entry in `content_ids`, sets the corresponding byte in
    /// `out_orphaned` to non-zero if the content is orphaned.
    #[inline]
    pub fn lookup_orphan_content(
        &self,
        content_ids: &[NcmContentId],
        out_orphaned: &mut [u8],
    ) -> Result<(), DispatchError> {
        cmif::content_meta_database::lookup_orphan_content(&self.0, content_ids, out_orphaned)
    }

    /// Commits changes (cmd 15).
    #[inline]
    pub fn commit(&self) -> Result<(), DispatchError> {
        cmif::content_meta_database::commit(&self.0)
    }

    /// Checks if content meta has a specific content (cmd 16).
    #[inline]
    pub fn has_content(
        &self,
        key: &NcmContentMetaKey,
        content_id: &NcmContentId,
    ) -> Result<bool, DispatchError> {
        cmif::content_meta_database::has_content(&self.0, key, content_id)
    }

    /// Lists content meta info (cmd 17).
    #[inline]
    pub fn list_content_meta_info(
        &self,
        key: &NcmContentMetaKey,
        start_index: i32,
        out_meta_info: &mut [NcmContentMetaInfo],
    ) -> Result<i32, DispatchError> {
        cmif::content_meta_database::list_content_meta_info(
            &self.0,
            key,
            start_index,
            out_meta_info,
        )
    }

    /// Gets attributes (cmd 18).
    #[inline]
    pub fn get_attributes(&self, key: &NcmContentMetaKey) -> Result<u8, DispatchError> {
        cmif::content_meta_database::get_attributes(&self.0, key)
    }

    /// Gets required application version (cmd 19, 2.0.0+).
    #[inline]
    pub fn get_required_application_version(
        &self,
        key: &NcmContentMetaKey,
    ) -> Result<u32, DispatchError> {
        cmif::content_meta_database::get_required_application_version(&self.0, key)
    }

    /// Gets content ID by type and ID offset (cmd 20, 5.0.0+).
    #[inline]
    pub fn get_content_id_by_type_and_id_offset(
        &self,
        key: &NcmContentMetaKey,
        content_type: NcmContentType,
        id_offset: u8,
    ) -> Result<NcmContentId, DispatchError> {
        cmif::content_meta_database::get_content_id_by_type_and_id_offset(
            &self.0,
            key,
            content_type,
            id_offset,
        )
    }

    /// Gets platform (cmd 26, 17.0.0+).
    #[inline]
    pub fn get_platform(&self, key: &NcmContentMetaKey) -> Result<u8, DispatchError> {
        cmif::content_meta_database::get_platform(&self.0, key)
    }
}

/// Connects to the NCM service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<NcmService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    Ok(NcmService(Session::new(handle, 0)))
}

/// Errors returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    /// SM lookup for `ncm` failed.
    #[error("failed to look up ncm service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
}
