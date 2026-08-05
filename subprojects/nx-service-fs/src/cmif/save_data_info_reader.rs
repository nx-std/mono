use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainObjectRef,
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::with_ipc_buffer,
    proto,
    savedata::SaveDataInfo,
};

pub(crate) fn read(
    object: DomainObjectRef<'_>,
    ctx: u32,
    buf: &mut [SaveDataInfo],
) -> Result<i64, DispatchError> {
    let buf_bytes = buf.as_mut_bytes();
    with_ipc_buffer(|ipc_buf| {
        let result = object
            .dispatch(proto::SAVE_DATA_INFO_READER_READ)
            .context(ctx)
            .out_size(size_of::<i64>())
            .out_buffer(buf_bytes, BufferAttr::HIPC_MAP_ALIAS)
            .send(ipc_buf)?;
        Ok(*result.value::<i64>())
    })
}
