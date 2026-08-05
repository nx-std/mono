use nx_sf::service::{
    DispatchError,
    DomainObjectRef,
    OutHandleAttr,
};

use crate::{
    dispatch::with_ipc_buffer,
    proto,
};

pub(crate) fn get_event_handle(
    object: DomainObjectRef<'_>,
    ctx: u32,
) -> Result<u32, DispatchError> {
    with_ipc_buffer(|ipc_buf| {
        let result = object
            .dispatch(proto::EVENT_NOTIFIER_GET_EVENT_HANDLE)
            .context(ctx)
            .out_handle(0, OutHandleAttr::Copy)
            .send(ipc_buf)?;

        Ok(result.copy_handles[0])
    })
}
