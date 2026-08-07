use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainObjectRef,
};
use zerocopy::IntoBytes as _;

use crate::{
    proto,
    savedata::SaveDataInfo,
};

pub(crate) fn read(
    object: DomainObjectRef<'_>,
    ctx: u32,
    buf: &mut [SaveDataInfo],
) -> Result<i64, DispatchError> {
    let buf_bytes = buf.as_mut_bytes();
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(proto::SAVE_DATA_INFO_READER_READ)
        .context(ctx)
        .out_size(size_of::<i64>())
        .out_buffer(buf_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<i64>())
}
