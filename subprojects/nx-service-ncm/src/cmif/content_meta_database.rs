//! IContentMetaDatabase sub-object commands.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Session};

use crate::{
    dispatch::{dispatch_in, dispatch_in_out, dispatch_no_io},
    proto,
    types::{
        GetContentIdByTypeAndIdOffsetIn, GetContentIdByTypeIn, HasContentIn, ListContentInfoIn,
        ListIn, ListOut, NcmApplicationContentMetaKey, NcmContentId, NcmContentInfo,
        NcmContentMetaInfo, NcmContentMetaKey, NcmContentMetaType, NcmContentType,
    },
};

/// Sets content meta (cmd 0).
pub(crate) fn set(
    service: &Session,
    key: &NcmContentMetaKey,
    data: &[u8],
) -> Result<(), DispatchError> {
    // SAFETY: `key` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let key_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const *key).cast::<u8>(),
            size_of::<NcmContentMetaKey>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::DB_SET)
        .in_raw(key_bytes)
        .in_buffer(data, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(())
}

/// Gets content meta (cmd 1).
pub(crate) fn get(
    service: &Session,
    key: &NcmContentMetaKey,
    out_data: &mut [u8],
) -> Result<u64, DispatchError> {
    // SAFETY: `key` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let key_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const *key).cast::<u8>(),
            size_of::<NcmContentMetaKey>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::DB_GET)
        .in_raw(key_bytes)
        .out_size(size_of::<u64>())
        .out_buffer(out_data, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    // SAFETY: response payload is at least size_of::<u64>() bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) })
}

/// Removes content meta (cmd 2).
pub(crate) fn remove(service: &Session, key: &NcmContentMetaKey) -> Result<(), DispatchError> {
    dispatch_in(service, proto::DB_REMOVE, *key)
}

/// Gets content ID by type (cmd 3).
pub(crate) fn get_content_id_by_type(
    service: &Session,
    key: &NcmContentMetaKey,
    content_type: NcmContentType,
) -> Result<NcmContentId, DispatchError> {
    dispatch_in_out(
        service,
        proto::DB_GET_CONTENT_ID_BY_TYPE,
        GetContentIdByTypeIn {
            content_type: content_type as u8,
            padding: [0; 7],
            key: *key,
        },
    )
}

/// Lists content info (cmd 4).
pub(crate) fn list_content_info(
    service: &Session,
    key: &NcmContentMetaKey,
    start_index: i32,
    out_info: &mut [NcmContentInfo],
) -> Result<i32, DispatchError> {
    let input = ListContentInfoIn {
        start_index,
        pad: 0,
        key: *key,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<ListContentInfoIn>(),
        )
    };
    // SAFETY: `out_info` is a valid `&mut` slice; viewing it as a byte slice
    // for the OUT buffer is sound, and the byte slice borrows `out_info`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            out_info.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(out_info),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::DB_LIST_CONTENT_INFO)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    // SAFETY: response payload is at least size_of::<i32>() bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

/// Lists content meta keys (cmd 5).
pub(crate) fn list(
    service: &Session,
    meta_type: NcmContentMetaType,
    id: u64,
    id_min: u64,
    id_max: u64,
    install_type: u8,
    out_keys: &mut [NcmContentMetaKey],
) -> Result<ListOut, DispatchError> {
    let input = ListIn {
        meta_type: meta_type as u8,
        install_type,
        padding: [0; 6],
        id,
        id_min,
        id_max,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<ListIn>())
    };
    // SAFETY: `out_keys` is a valid `&mut` slice; viewing it as a byte slice
    // for the OUT buffer is sound, and the byte slice borrows `out_keys`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            out_keys.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(out_keys),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::DB_LIST)
        .in_raw(in_bytes)
        .out_size(size_of::<ListOut>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    // SAFETY: response payload is at least size_of::<ListOut>() bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<ListOut>()) })
}

/// Gets the latest content meta key (cmd 6).
pub(crate) fn get_latest_content_meta_key(
    service: &Session,
    id: u64,
) -> Result<NcmContentMetaKey, DispatchError> {
    dispatch_in_out(service, proto::DB_GET_LATEST_CONTENT_META_KEY, id)
}

/// Lists application content meta keys (cmd 7).
pub(crate) fn list_application(
    service: &Session,
    meta_type: NcmContentMetaType,
    out_keys: &mut [NcmApplicationContentMetaKey],
) -> Result<ListOut, DispatchError> {
    let input = meta_type as u8;
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its single byte as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<u8>()) };
    // SAFETY: `out_keys` is a valid `&mut` slice; viewing it as a byte slice
    // for the OUT buffer is sound, and the byte slice borrows `out_keys`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            out_keys.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(out_keys),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::DB_LIST_APPLICATION)
        .in_raw(in_bytes)
        .out_size(size_of::<ListOut>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    // SAFETY: response payload is at least size_of::<ListOut>() bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<ListOut>()) })
}

/// Checks if a content meta key exists (cmd 8).
pub(crate) fn has(service: &Session, key: &NcmContentMetaKey) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_in_out(service, proto::DB_HAS, *key)?;
    Ok(out & 1 != 0)
}

/// Checks if all content meta keys exist (cmd 9).
pub(crate) fn has_all(
    service: &Session,
    keys: &[NcmContentMetaKey],
) -> Result<bool, DispatchError> {
    // SAFETY: `keys` is a valid `&` slice; viewing it as a byte slice
    // for the IN buffer is sound, and the byte slice borrows `keys`.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(keys.as_ptr().cast::<u8>(), core::mem::size_of_val(keys))
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::DB_HAS_ALL)
        .out_size(size_of::<u8>())
        .in_buffer(in_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    // SAFETY: response payload is at least size_of::<u8>() bytes.
    let out = unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    Ok(out & 1 != 0)
}

/// Gets the size of a content meta (cmd 10).
pub(crate) fn get_size(service: &Session, key: &NcmContentMetaKey) -> Result<u64, DispatchError> {
    dispatch_in_out(service, proto::DB_GET_SIZE, *key)
}

/// Gets the required system version (cmd 11).
pub(crate) fn get_required_system_version(
    service: &Session,
    key: &NcmContentMetaKey,
) -> Result<u32, DispatchError> {
    dispatch_in_out(service, proto::DB_GET_REQUIRED_SYSTEM_VERSION, *key)
}

/// Gets the patch content meta ID (cmd 12).
pub(crate) fn get_patch_content_meta_id(
    service: &Session,
    key: &NcmContentMetaKey,
) -> Result<u64, DispatchError> {
    dispatch_in_out(service, proto::DB_GET_PATCH_CONTENT_META_ID, *key)
}

/// Disables forcibly (cmd 13).
pub(crate) fn disable_forcibly(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::DB_DISABLE_FORCIBLY)
}

/// Looks up orphan content (cmd 14).
pub(crate) fn lookup_orphan_content(
    service: &Session,
    content_ids: &[NcmContentId],
    out_orphaned: &mut [u8],
) -> Result<(), DispatchError> {
    // SAFETY: `content_ids` is a valid `&` slice; viewing it as a byte slice
    // for the IN buffer is sound, and the byte slice borrows `content_ids`.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            content_ids.as_ptr().cast::<u8>(),
            core::mem::size_of_val(content_ids),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::DB_LOOKUP_ORPHAN_CONTENT)
        .out_buffer(out_orphaned, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(in_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(())
}

/// Commits changes (cmd 15).
pub(crate) fn commit(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::DB_COMMIT)
}

/// Checks if content meta has a specific content (cmd 16).
pub(crate) fn has_content(
    service: &Session,
    key: &NcmContentMetaKey,
    content_id: &NcmContentId,
) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_in_out(
        service,
        proto::DB_HAS_CONTENT,
        HasContentIn {
            content_id: *content_id,
            key: *key,
        },
    )?;
    Ok(out & 1 != 0)
}

/// Lists content meta info (cmd 17).
pub(crate) fn list_content_meta_info(
    service: &Session,
    key: &NcmContentMetaKey,
    start_index: i32,
    out_meta_info: &mut [NcmContentMetaInfo],
) -> Result<i32, DispatchError> {
    let input = ListContentInfoIn {
        start_index,
        pad: 0,
        key: *key,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<ListContentInfoIn>(),
        )
    };
    // SAFETY: `out_meta_info` is a valid `&mut` slice; viewing it as a byte
    // slice for the OUT buffer is sound, and the byte slice borrows `out_meta_info`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            out_meta_info.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(out_meta_info),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::DB_LIST_CONTENT_META_INFO)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    // SAFETY: response payload is at least size_of::<i32>() bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

/// Gets attributes (cmd 18).
pub(crate) fn get_attributes(
    service: &Session,
    key: &NcmContentMetaKey,
) -> Result<u8, DispatchError> {
    dispatch_in_out(service, proto::DB_GET_ATTRIBUTES, *key)
}

/// Gets required application version (cmd 19, 2.0.0+).
pub(crate) fn get_required_application_version(
    service: &Session,
    key: &NcmContentMetaKey,
) -> Result<u32, DispatchError> {
    dispatch_in_out(service, proto::DB_GET_REQUIRED_APPLICATION_VERSION, *key)
}

/// Gets content ID by type and ID offset (cmd 20, 5.0.0+).
pub(crate) fn get_content_id_by_type_and_id_offset(
    service: &Session,
    key: &NcmContentMetaKey,
    content_type: NcmContentType,
    id_offset: u8,
) -> Result<NcmContentId, DispatchError> {
    dispatch_in_out(
        service,
        proto::DB_GET_CONTENT_ID_BY_TYPE_AND_ID_OFFSET,
        GetContentIdByTypeAndIdOffsetIn {
            content_type: content_type as u8,
            id_offset,
            padding: [0; 6],
            key: *key,
        },
    )
}

/// Gets platform (cmd 26, 17.0.0+).
pub(crate) fn get_platform(
    service: &Session,
    key: &NcmContentMetaKey,
) -> Result<u8, DispatchError> {
    dispatch_in_out(service, proto::DB_GET_PLATFORM, *key)
}
