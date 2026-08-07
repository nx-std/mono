//! CMIF dispatch helpers shared across the `cmif` module.

use core::mem::size_of;

use nx_sf::service::{
    DispatchError,
    Session,
};
use zerocopy::IntoBytes as _;

/// CMIF request with no input payload and no output payload.
#[inline]
pub(crate) fn dispatch_no_io(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();
    service.dispatch(cmd_id).send(&mut ipc_buf).map(|_| ())
}

/// CMIF request with a `u64` input and no output payload.
#[inline]
pub(crate) fn dispatch_in_u64(
    service: &Session,
    cmd_id: u32,
    value: u64,
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(value.as_bytes())
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// CMIF request with a `u64` input and a `u32` output.
#[inline]
pub(crate) fn dispatch_in_u64_out_u32(
    service: &Session,
    cmd_id: u32,
    value: u64,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(value.as_bytes())
        .out_size(size_of::<u32>())
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u32>())
}
