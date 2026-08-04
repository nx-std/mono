use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, DomainObjectRef};

use crate::{
    dispatch::{dispatch_in, dispatch_in_out, dispatch_no_io, dispatch_out_i64},
    proto,
    types::*,
};

fn as_in_bytes<I: Copy>(input: &I) -> &[u8] {
    unsafe { core::slice::from_raw_parts((&raw const *input).cast::<u8>(), size_of::<I>()) }
}

pub(crate) fn read(
    object: DomainObjectRef<'_>,
    ctx: u32,
    offset: i64,
    buf: &mut [u8],
    read_size: u64,
) -> Result<(), DispatchError> {
    let input = StorageReadWriteIn {
        offset,
        size: read_size,
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(proto::STORAGE_READ)
        .context(ctx)
        .in_raw(as_in_bytes(&input))
        .out_buffer(
            buf,
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn write(
    object: DomainObjectRef<'_>,
    ctx: u32,
    offset: i64,
    buf: &[u8],
    write_size: u64,
) -> Result<(), DispatchError> {
    let input = StorageReadWriteIn {
        offset,
        size: write_size,
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(proto::STORAGE_WRITE)
        .context(ctx)
        .in_raw(as_in_bytes(&input))
        .in_buffer(
            buf,
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn flush(object: DomainObjectRef<'_>, ctx: u32) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::STORAGE_FLUSH, ctx)
}

pub(crate) fn set_size(
    object: DomainObjectRef<'_>,
    ctx: u32,
    size: i64,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::STORAGE_SET_SIZE, ctx, size)
}

pub(crate) fn get_size(object: DomainObjectRef<'_>, ctx: u32) -> Result<i64, DispatchError> {
    dispatch_out_i64(object, proto::STORAGE_GET_SIZE, ctx)
}

pub(crate) fn operate_range(
    object: DomainObjectRef<'_>,
    ctx: u32,
    op_id: u32,
    off: i64,
    len: i64,
) -> Result<RangeInfo, DispatchError> {
    let input = OperateRangeIn {
        op_id,
        pad: 0,
        off,
        len,
    };
    dispatch_in_out(object, proto::STORAGE_OPERATE_RANGE, ctx, input)
}
