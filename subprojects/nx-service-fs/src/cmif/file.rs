use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainObjectRef,
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::{
        dispatch_in,
        dispatch_in_out,
        dispatch_no_io,
        dispatch_out_i64,
        with_ipc_buffer,
    },
    file::{
        FileReadIn,
        FileWriteIn,
    },
    proto,
    range::{
        OperateRangeIn,
        RangeInfo,
    },
};

pub(crate) fn read(
    object: DomainObjectRef<'_>,
    ctx: u32,
    offset: i64,
    buf: &mut [u8],
    read_size: u64,
    option: u32,
) -> Result<u64, DispatchError> {
    let input = FileReadIn {
        option,
        pad: 0,
        offset,
        read_size,
    };
    with_ipc_buffer(|ipc_buf| {
        let result = object
            .dispatch(proto::FILE_READ)
            .context(ctx)
            .in_raw(input.as_bytes())
            .out_size(size_of::<u64>())
            .out_buffer(
                buf,
                BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE),
            )
            .send(ipc_buf)?;
        Ok(*result.value::<u64>())
    })
}

pub(crate) fn write(
    object: DomainObjectRef<'_>,
    ctx: u32,
    offset: i64,
    buf: &[u8],
    write_size: u64,
    option: u32,
) -> Result<(), DispatchError> {
    let input = FileWriteIn {
        option,
        pad: 0,
        offset,
        write_size,
    };
    with_ipc_buffer(|ipc_buf| {
        object
            .dispatch(proto::FILE_WRITE)
            .context(ctx)
            .in_raw(input.as_bytes())
            .in_buffer(
                buf,
                BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE),
            )
            .send(ipc_buf)
            .map(|_| ())
    })
}

pub(crate) fn flush(object: DomainObjectRef<'_>, ctx: u32) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::FILE_FLUSH, ctx)
}

pub(crate) fn set_size(
    object: DomainObjectRef<'_>,
    ctx: u32,
    size: i64,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::FILE_SET_SIZE, ctx, size)
}

pub(crate) fn get_size(object: DomainObjectRef<'_>, ctx: u32) -> Result<i64, DispatchError> {
    dispatch_out_i64(object, proto::FILE_GET_SIZE, ctx)
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
    dispatch_in_out(object, proto::FILE_OPERATE_RANGE, ctx, input)
}
