use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainObjectRef,
};
use zerocopy::IntoBytes as _;

use crate::{
    directory::DirectoryEntry,
    dispatch::dispatch_out_i64,
    proto,
};

pub(crate) fn read(
    object: DomainObjectRef<'_>,
    ctx: u32,
    buf: &mut [DirectoryEntry],
) -> Result<i64, DispatchError> {
    let buf_bytes = buf.as_mut_bytes();
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::DIR_READ)
        .context(ctx)
        .out_size(size_of::<i64>())
        .out_buffer(buf_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<i64>())
}

pub(crate) fn get_entry_count(object: DomainObjectRef<'_>, ctx: u32) -> Result<i64, DispatchError> {
    dispatch_out_i64(object, proto::DIR_GET_ENTRY_COUNT, ctx)
}
