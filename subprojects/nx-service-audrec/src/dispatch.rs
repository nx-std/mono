//! CMIF dispatch helpers shared across the `cmif` module.

use nx_sf::service::{
    DispatchError,
    Session,
};

/// CMIF request with no input payload and no output payload.
#[inline]
pub(crate) fn dispatch_no_io(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service.dispatch(cmd_id).send(&mut ipc_buf).map(|_| ())
}
