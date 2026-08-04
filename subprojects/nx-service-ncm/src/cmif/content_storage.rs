//! IContentStorage sub-object commands.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Session,
};

use crate::{
    dispatch::{
        dispatch_in,
        dispatch_in_out,
        dispatch_no_io,
        dispatch_out,
    },
    proto,
    types::{
        CreatePlaceHolderIn,
        CreatePlaceHolderLegacyIn,
        FS_MAX_PATH,
        FsContentAttributes,
        GetProgramIdIn,
        GetRightsIdFromContentIdIn,
        GetRightsIdFromPlaceHolderIdIn,
        GetRightsIdWithCacheIn,
        GetRightsIdWithCacheLegacyIn,
        NcmContentId,
        NcmPlaceHolderId,
        NcmRightsId,
        ReadContentIdFileIn,
        RegisterIn,
        RegisterLegacyIn,
        RevertToPlaceHolderIn,
        RevertToPlaceHolderLegacyIn,
        SetPlaceHolderSizeIn,
        WriteContentForDebugIn,
        WritePlaceHolderIn,
    },
};

/// Generates a placeholder ID (cmd 0).
pub(crate) fn generate_placeholder_id(
    service: &Session,
) -> Result<NcmPlaceHolderId, DispatchError> {
    dispatch_out(service, proto::CS_GENERATE_PLACEHOLDER_ID)
}

/// Creates a placeholder, pre-16.0.0 field ordering (cmd 1).
pub(crate) fn create_placeholder_legacy(
    service: &Session,
    content_id: &NcmContentId,
    placeholder_id: &NcmPlaceHolderId,
    size: i64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::CS_CREATE_PLACEHOLDER,
        CreatePlaceHolderLegacyIn {
            content_id: *content_id,
            placeholder_id: *placeholder_id,
            size,
        },
    )
}

/// Creates a placeholder, 16.0.0+ field ordering (cmd 1).
pub(crate) fn create_placeholder(
    service: &Session,
    content_id: &NcmContentId,
    placeholder_id: &NcmPlaceHolderId,
    size: i64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::CS_CREATE_PLACEHOLDER,
        CreatePlaceHolderIn {
            placeholder_id: *placeholder_id,
            content_id: *content_id,
            size,
        },
    )
}

/// Deletes a placeholder (cmd 2).
pub(crate) fn delete_placeholder(
    service: &Session,
    placeholder_id: &NcmPlaceHolderId,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::CS_DELETE_PLACEHOLDER, *placeholder_id)
}

/// Checks if a placeholder exists (cmd 3).
pub(crate) fn has_placeholder(
    service: &Session,
    placeholder_id: &NcmPlaceHolderId,
) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_in_out(service, proto::CS_HAS_PLACEHOLDER, *placeholder_id)?;
    Ok(out & 1 != 0)
}

/// Writes data to a placeholder (cmd 4).
pub(crate) fn write_placeholder(
    service: &Session,
    placeholder_id: &NcmPlaceHolderId,
    offset: u64,
    data: &[u8],
) -> Result<(), DispatchError> {
    let input = WritePlaceHolderIn {
        placeholder_id: *placeholder_id,
        offset,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<WritePlaceHolderIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::CS_WRITE_PLACEHOLDER)
        .in_raw(in_bytes)
        .in_buffer(data, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(())
}

/// Registers a content ID from a placeholder, pre-16.0.0 field ordering (cmd 5).
pub(crate) fn register_legacy(
    service: &Session,
    content_id: &NcmContentId,
    placeholder_id: &NcmPlaceHolderId,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::CS_REGISTER,
        RegisterLegacyIn {
            content_id: *content_id,
            placeholder_id: *placeholder_id,
        },
    )
}

/// Registers a content ID from a placeholder, 16.0.0+ field ordering (cmd 5).
pub(crate) fn register(
    service: &Session,
    content_id: &NcmContentId,
    placeholder_id: &NcmPlaceHolderId,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::CS_REGISTER,
        RegisterIn {
            placeholder_id: *placeholder_id,
            content_id: *content_id,
        },
    )
}

/// Deletes a content ID (cmd 6).
pub(crate) fn delete(service: &Session, content_id: &NcmContentId) -> Result<(), DispatchError> {
    dispatch_in(service, proto::CS_DELETE, *content_id)
}

/// Checks if a content ID exists (cmd 7).
pub(crate) fn has(service: &Session, content_id: &NcmContentId) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_in_out(service, proto::CS_HAS, *content_id)?;
    Ok(out & 1 != 0)
}

/// Gets the filesystem path for a content ID (cmd 8).
pub(crate) fn get_path(
    service: &Session,
    content_id: &NcmContentId,
    out_path: &mut [u8; FS_MAX_PATH],
) -> Result<(), DispatchError> {
    // SAFETY: `content_id` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const *content_id).cast::<u8>(),
            size_of::<NcmContentId>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::CS_GET_PATH)
        .in_raw(in_bytes)
        .out_buffer(
            out_path.as_mut_slice(),
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)?;
    Ok(())
}

/// Gets the filesystem path for a placeholder (cmd 9).
pub(crate) fn get_placeholder_path(
    service: &Session,
    placeholder_id: &NcmPlaceHolderId,
    out_path: &mut [u8; FS_MAX_PATH],
) -> Result<(), DispatchError> {
    // SAFETY: `placeholder_id` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const *placeholder_id).cast::<u8>(),
            size_of::<NcmPlaceHolderId>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::CS_GET_PLACEHOLDER_PATH)
        .in_raw(in_bytes)
        .out_buffer(
            out_path.as_mut_slice(),
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)?;
    Ok(())
}

/// Cleans up all placeholders (cmd 10).
pub(crate) fn cleanup_all_placeholder(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::CS_CLEANUP_ALL_PLACEHOLDER)
}

/// Lists placeholders (cmd 11).
pub(crate) fn list_placeholder(
    service: &Session,
    out_ids: &mut [NcmPlaceHolderId],
) -> Result<i32, DispatchError> {
    // SAFETY: `out_ids` is a valid `&mut` slice; viewing it as a byte slice
    // for the OUT buffer is sound, and the byte slice borrows `out_ids`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            out_ids.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(out_ids),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::CS_LIST_PLACEHOLDER)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    // SAFETY: response payload is at least size_of::<i32>() bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

/// Gets the content count (cmd 12).
pub(crate) fn get_content_count(service: &Session) -> Result<i32, DispatchError> {
    dispatch_out(service, proto::CS_GET_CONTENT_COUNT)
}

/// Lists content IDs (cmd 13).
pub(crate) fn list_content_id(
    service: &Session,
    out_ids: &mut [NcmContentId],
    start_offset: i32,
) -> Result<i32, DispatchError> {
    // SAFETY: `start_offset` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const start_offset).cast::<u8>(), size_of::<i32>())
    };
    // SAFETY: `out_ids` is a valid `&mut` slice; viewing it as a byte slice
    // for the OUT buffer is sound, and the byte slice borrows `out_ids`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            out_ids.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(out_ids),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::CS_LIST_CONTENT_ID)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    // SAFETY: response payload is at least size_of::<i32>() bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

/// Gets the size of a content ID (cmd 14).
pub(crate) fn get_size_from_content_id(
    service: &Session,
    content_id: &NcmContentId,
) -> Result<i64, DispatchError> {
    dispatch_in_out(service, proto::CS_GET_SIZE_FROM_CONTENT_ID, *content_id)
}

/// Disables forcibly (cmd 15).
pub(crate) fn disable_forcibly(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::CS_DISABLE_FORCIBLY)
}

/// Reverts to a placeholder, pre-16.0.0 field ordering (cmd 16, 2.0.0+).
pub(crate) fn revert_to_placeholder_legacy(
    service: &Session,
    placeholder_id: &NcmPlaceHolderId,
    old_content_id: &NcmContentId,
    new_content_id: &NcmContentId,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::CS_REVERT_TO_PLACEHOLDER,
        RevertToPlaceHolderLegacyIn {
            old_content_id: *old_content_id,
            new_content_id: *new_content_id,
            placeholder_id: *placeholder_id,
        },
    )
}

/// Reverts to a placeholder, 16.0.0+ field ordering (cmd 16).
pub(crate) fn revert_to_placeholder(
    service: &Session,
    placeholder_id: &NcmPlaceHolderId,
    old_content_id: &NcmContentId,
    new_content_id: &NcmContentId,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::CS_REVERT_TO_PLACEHOLDER,
        RevertToPlaceHolderIn {
            placeholder_id: *placeholder_id,
            old_content_id: *old_content_id,
            new_content_id: *new_content_id,
        },
    )
}

/// Sets the placeholder size (cmd 17, 2.0.0+).
pub(crate) fn set_placeholder_size(
    service: &Session,
    placeholder_id: &NcmPlaceHolderId,
    size: i64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::CS_SET_PLACEHOLDER_SIZE,
        SetPlaceHolderSizeIn {
            placeholder_id: *placeholder_id,
            size,
        },
    )
}

/// Reads content ID file data (cmd 18, 2.0.0+).
pub(crate) fn read_content_id_file(
    service: &Session,
    content_id: &NcmContentId,
    offset: i64,
    out_data: &mut [u8],
) -> Result<(), DispatchError> {
    let input = ReadContentIdFileIn {
        content_id: *content_id,
        offset,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<ReadContentIdFileIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::CS_READ_CONTENT_ID_FILE)
        .in_raw(in_bytes)
        .out_buffer(out_data, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(())
}

/// Gets rights ID from a placeholder ID, pre-3.0.0 (cmd 19, 2.0.0+).
///
/// Returns only the `FsRightsId` portion (no key_generation).
pub(crate) fn get_rights_id_from_placeholder_id_legacy(
    service: &Session,
    placeholder_id: &NcmPlaceHolderId,
    attr: FsContentAttributes,
) -> Result<[u8; 0x10], DispatchError> {
    let input = GetRightsIdFromPlaceHolderIdIn {
        placeholder_id: *placeholder_id,
        attr: attr as u8,
    };
    dispatch_in_out(service, proto::CS_GET_RIGHTS_ID_FROM_PLACEHOLDER_ID, input)
}

/// Gets rights ID from a placeholder ID, 3.0.0+ (cmd 19).
pub(crate) fn get_rights_id_from_placeholder_id(
    service: &Session,
    placeholder_id: &NcmPlaceHolderId,
    attr: FsContentAttributes,
) -> Result<NcmRightsId, DispatchError> {
    let input = GetRightsIdFromPlaceHolderIdIn {
        placeholder_id: *placeholder_id,
        attr: attr as u8,
    };
    dispatch_in_out(service, proto::CS_GET_RIGHTS_ID_FROM_PLACEHOLDER_ID, input)
}

/// Gets rights ID from a content ID, pre-3.0.0 (cmd 20, 2.0.0+).
pub(crate) fn get_rights_id_from_content_id_legacy(
    service: &Session,
    content_id: &NcmContentId,
    attr: FsContentAttributes,
) -> Result<[u8; 0x10], DispatchError> {
    let input = GetRightsIdFromContentIdIn {
        content_id: *content_id,
        attr: attr as u8,
    };
    dispatch_in_out(service, proto::CS_GET_RIGHTS_ID_FROM_CONTENT_ID, input)
}

/// Gets rights ID from a content ID, 3.0.0+ (cmd 20).
pub(crate) fn get_rights_id_from_content_id(
    service: &Session,
    content_id: &NcmContentId,
    attr: FsContentAttributes,
) -> Result<NcmRightsId, DispatchError> {
    let input = GetRightsIdFromContentIdIn {
        content_id: *content_id,
        attr: attr as u8,
    };
    dispatch_in_out(service, proto::CS_GET_RIGHTS_ID_FROM_CONTENT_ID, input)
}

/// Writes content data for debug (cmd 21, 2.0.0+).
pub(crate) fn write_content_for_debug(
    service: &Session,
    content_id: &NcmContentId,
    offset: i64,
    data: &[u8],
) -> Result<(), DispatchError> {
    let input = WriteContentForDebugIn {
        content_id: *content_id,
        offset,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<WriteContentForDebugIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::CS_WRITE_CONTENT_FOR_DEBUG)
        .in_raw(in_bytes)
        .in_buffer(data, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(())
}

/// Gets free space size (cmd 22, 2.0.0+).
pub(crate) fn get_free_space_size(service: &Session) -> Result<i64, DispatchError> {
    dispatch_out(service, proto::CS_GET_FREE_SPACE_SIZE)
}

/// Gets total space size (cmd 23, 2.0.0+).
pub(crate) fn get_total_space_size(service: &Session) -> Result<i64, DispatchError> {
    dispatch_out(service, proto::CS_GET_TOTAL_SPACE_SIZE)
}

/// Flushes placeholder data (cmd 24, 3.0.0+).
pub(crate) fn flush_placeholder(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::CS_FLUSH_PLACEHOLDER)
}

/// Gets size from a placeholder ID (cmd 25, 4.0.0+).
pub(crate) fn get_size_from_placeholder_id(
    service: &Session,
    placeholder_id: &NcmPlaceHolderId,
) -> Result<i64, DispatchError> {
    dispatch_in_out(
        service,
        proto::CS_GET_SIZE_FROM_PLACEHOLDER_ID,
        *placeholder_id,
    )
}

/// Repairs invalid file attributes (cmd 26, 4.0.0+).
pub(crate) fn repair_invalid_file_attribute(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::CS_REPAIR_INVALID_FILE_ATTRIBUTE)
}

/// Gets rights ID from placeholder with cache, pre-16.0.0 (cmd 27, 8.0.0+).
pub(crate) fn get_rights_id_from_placeholder_id_with_cache_legacy(
    service: &Session,
    placeholder_id: &NcmPlaceHolderId,
    cache_content_id: &NcmContentId,
) -> Result<NcmRightsId, DispatchError> {
    dispatch_in_out(
        service,
        proto::CS_GET_RIGHTS_ID_FROM_PLACEHOLDER_ID_WITH_CACHE,
        GetRightsIdWithCacheLegacyIn {
            cache_content_id: *cache_content_id,
            placeholder_id: *placeholder_id,
        },
    )
}

/// Gets rights ID from placeholder with cache, 16.0.0+ (cmd 27).
pub(crate) fn get_rights_id_from_placeholder_id_with_cache(
    service: &Session,
    placeholder_id: &NcmPlaceHolderId,
    cache_content_id: &NcmContentId,
    attr: FsContentAttributes,
) -> Result<NcmRightsId, DispatchError> {
    dispatch_in_out(
        service,
        proto::CS_GET_RIGHTS_ID_FROM_PLACEHOLDER_ID_WITH_CACHE,
        GetRightsIdWithCacheIn {
            placeholder_id: *placeholder_id,
            cache_content_id: *cache_content_id,
            attr: attr as u8,
        },
    )
}

/// Registers a path for content (cmd 28, 13.0.0+).
pub(crate) fn register_path(
    service: &Session,
    content_id: &NcmContentId,
    path: &[u8; FS_MAX_PATH],
) -> Result<(), DispatchError> {
    // SAFETY: `content_id` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const *content_id).cast::<u8>(),
            size_of::<NcmContentId>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::CS_REGISTER_PATH)
        .in_raw(in_bytes)
        .in_buffer(path.as_slice(), BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;
    Ok(())
}

/// Clears registered paths (cmd 29, 13.0.0+).
pub(crate) fn clear_registered_path(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::CS_CLEAR_REGISTERED_PATH)
}

/// Gets program ID from content ID (cmd 30, 17.0.0+).
pub(crate) fn get_program_id(
    service: &Session,
    content_id: &NcmContentId,
    attr: FsContentAttributes,
) -> Result<u64, DispatchError> {
    dispatch_in_out(
        service,
        proto::CS_GET_PROGRAM_ID,
        GetProgramIdIn {
            content_id: *content_id,
            attr: attr as u8,
        },
    )
}
