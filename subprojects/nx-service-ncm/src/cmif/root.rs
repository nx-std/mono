//! IContentManager root service commands.

use nx_sf::service::{
    DispatchError,
    Session,
};

use crate::{
    dispatch::{
        OpenSubObjectError,
        dispatch_in,
        dispatch_in_u8_out_object,
        dispatch_no_io,
    },
    proto,
    types::NcmStorageId,
};

/// Creates a content storage for the given storage ID (cmd 0).
pub(crate) fn create_content_storage(
    service: &Session,
    storage_id: NcmStorageId,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::CREATE_CONTENT_STORAGE, storage_id as u8)
}

/// Creates a content meta database for the given storage ID (cmd 1).
pub(crate) fn create_content_meta_database(
    service: &Session,
    storage_id: NcmStorageId,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::CREATE_CONTENT_META_DATABASE,
        storage_id as u8,
    )
}

/// Verifies a content storage (cmd 2).
pub(crate) fn verify_content_storage(
    service: &Session,
    storage_id: NcmStorageId,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::VERIFY_CONTENT_STORAGE, storage_id as u8)
}

/// Verifies a content meta database (cmd 3).
pub(crate) fn verify_content_meta_database(
    service: &Session,
    storage_id: NcmStorageId,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::VERIFY_CONTENT_META_DATABASE,
        storage_id as u8,
    )
}

/// Opens a content storage sub-object (cmd 4). Returns a move handle.
pub(crate) fn open_content_storage(
    service: &Session,
    storage_id: NcmStorageId,
) -> Result<u32, OpenSubObjectError> {
    dispatch_in_u8_out_object(service, proto::OPEN_CONTENT_STORAGE, storage_id as u8)
}

/// Opens a content meta database sub-object (cmd 5). Returns a move handle.
pub(crate) fn open_content_meta_database(
    service: &Session,
    storage_id: NcmStorageId,
) -> Result<u32, OpenSubObjectError> {
    dispatch_in_u8_out_object(service, proto::OPEN_CONTENT_META_DATABASE, storage_id as u8)
}

/// Closes content storage forcibly (cmd 6, pre-2.0.0).
pub(crate) fn close_content_storage_forcibly(
    service: &Session,
    storage_id: NcmStorageId,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::CLOSE_CONTENT_STORAGE_FORCIBLY,
        storage_id as u8,
    )
}

/// Closes content meta database forcibly (cmd 7, pre-2.0.0).
pub(crate) fn close_content_meta_database_forcibly(
    service: &Session,
    storage_id: NcmStorageId,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::CLOSE_CONTENT_META_DATABASE_FORCIBLY,
        storage_id as u8,
    )
}

/// Cleans up content meta database (cmd 8).
pub(crate) fn cleanup_content_meta_database(
    service: &Session,
    storage_id: NcmStorageId,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::CLEANUP_CONTENT_META_DATABASE,
        storage_id as u8,
    )
}

/// Activates a content storage (cmd 9, 2.0.0+).
pub(crate) fn activate_content_storage(
    service: &Session,
    storage_id: NcmStorageId,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::ACTIVATE_CONTENT_STORAGE, storage_id as u8)
}

/// Inactivates a content storage (cmd 10, 2.0.0+).
pub(crate) fn inactivate_content_storage(
    service: &Session,
    storage_id: NcmStorageId,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::INACTIVATE_CONTENT_STORAGE, storage_id as u8)
}

/// Activates a content meta database (cmd 11, 2.0.0+).
pub(crate) fn activate_content_meta_database(
    service: &Session,
    storage_id: NcmStorageId,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::ACTIVATE_CONTENT_META_DATABASE,
        storage_id as u8,
    )
}

/// Inactivates a content meta database (cmd 12, 2.0.0+).
pub(crate) fn inactivate_content_meta_database(
    service: &Session,
    storage_id: NcmStorageId,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::INACTIVATE_CONTENT_META_DATABASE,
        storage_id as u8,
    )
}

/// Invalidates the rights ID cache (cmd 13, 9.0.0+).
pub(crate) fn invalidate_rights_id_cache(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::INVALIDATE_RIGHTS_ID_CACHE)
}

/// Activates FS content storage (cmd 15, 16.0.0+).
pub(crate) fn activate_fs_content_storage(
    service: &Session,
    fs_storage_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::ACTIVATE_FS_CONTENT_STORAGE, fs_storage_id)
}
