use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainObjectRef,
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::{
        dispatch_no_io,
        dispatch_out,
    },
    file::{
        CreateFileIn,
        TimeStampRaw,
    },
    filesystem::FileSystemAttribute,
    path::FS_MAX_PATH,
    proto,
};

pub(crate) fn create_file(
    object: DomainObjectRef<'_>,
    ctx: u32,
    path: &[u8; FS_MAX_PATH],
    size: i64,
    option: u32,
) -> Result<(), DispatchError> {
    let input = CreateFileIn {
        option,
        _pad: 0,
        size,
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(proto::FS_CREATE_FILE)
        .context(ctx)
        .in_raw(input.as_bytes())
        .in_buffer(path, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn cmd_with_path(
    object: DomainObjectRef<'_>,
    ctx: u32,
    cmd_id: u32,
    path: &[u8; FS_MAX_PATH],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(cmd_id)
        .context(ctx)
        .in_buffer(path, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn cmd_with_two_paths(
    object: DomainObjectRef<'_>,
    ctx: u32,
    cmd_id: u32,
    cur_path: &[u8; FS_MAX_PATH],
    new_path: &[u8; FS_MAX_PATH],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(cmd_id)
        .context(ctx)
        .in_buffer(cur_path, BufferAttr::HIPC_POINTER)
        .in_buffer(new_path, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn get_entry_type(
    object: DomainObjectRef<'_>,
    ctx: u32,
    path: &[u8; FS_MAX_PATH],
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::FS_GET_ENTRY_TYPE)
        .context(ctx)
        .out_size(size_of::<u32>())
        .in_buffer(path, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u32>())
}

pub(crate) fn open_file(
    object: DomainObjectRef<'_>,
    ctx: u32,
    path: &[u8; FS_MAX_PATH],
    mode: u32,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = object
        .dispatch(proto::FS_OPEN_FILE)
        .context(ctx)
        .in_raw(mode.as_bytes())
        .in_buffer(path, BufferAttr::HIPC_POINTER)
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let obj = result.take_object(0).expect("server returned file object");
    Ok(obj.into_raw_object_id())
}

pub(crate) fn open_directory(
    object: DomainObjectRef<'_>,
    ctx: u32,
    path: &[u8; FS_MAX_PATH],
    mode: u32,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = object
        .dispatch(proto::FS_OPEN_DIRECTORY)
        .context(ctx)
        .in_raw(mode.as_bytes())
        .in_buffer(path, BufferAttr::HIPC_POINTER)
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let obj = result
        .take_object(0)
        .expect("server returned directory object");
    Ok(obj.into_raw_object_id())
}

pub(crate) fn commit(object: DomainObjectRef<'_>, ctx: u32) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::FS_COMMIT, ctx)
}

pub(crate) fn get_space(
    object: DomainObjectRef<'_>,
    ctx: u32,
    cmd_id: u32,
    path: &[u8; FS_MAX_PATH],
) -> Result<i64, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(cmd_id)
        .context(ctx)
        .out_size(size_of::<u64>())
        .in_buffer(path, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<i64>())
}

pub(crate) fn get_file_time_stamp_raw(
    object: DomainObjectRef<'_>,
    ctx: u32,
    path: &[u8; FS_MAX_PATH],
) -> Result<TimeStampRaw, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::FS_GET_FILE_TIME_STAMP_RAW)
        .context(ctx)
        .out_size(size_of::<TimeStampRaw>())
        .in_buffer(path, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<TimeStampRaw>())
}

pub(crate) fn query_entry(
    object: DomainObjectRef<'_>,
    ctx: u32,
    path: &[u8; FS_MAX_PATH],
    query_id: u32,
    in_buf: &[u8],
    out_buf: &mut [u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(proto::FS_QUERY_ENTRY)
        .context(ctx)
        .in_raw(query_id.as_bytes())
        .in_buffer(path, BufferAttr::HIPC_POINTER)
        .in_buffer(
            in_buf,
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE),
        )
        .out_buffer(
            out_buf,
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn get_file_system_attribute(
    object: DomainObjectRef<'_>,
    ctx: u32,
) -> Result<FileSystemAttribute, DispatchError> {
    dispatch_out(object, proto::FS_GET_FILE_SYSTEM_ATTRIBUTE, ctx)
}
