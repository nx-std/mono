use core::mem::ManuallyDrop;

use nx_sf::service::{DispatchError, DomainObject, OutHandleAttr};

use crate::proto;

pub(crate) fn get_event_handle(
    object: &ManuallyDrop<DomainObject<'_>>,
    ctx: u32,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = object
        .dispatch(proto::EVENT_NOTIFIER_GET_EVENT_HANDLE)
        .context(ctx)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)?;

    Ok(result.copy_handles[0])
}
