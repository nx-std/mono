//! CMIF dispatch helpers shared across the `cmif` module.

use core::mem::size_of;

use nx_sf::service::{
    DispatchError,
    Session,
};

/// CMIF request with no input payload and no output payload.
#[inline]
pub(crate) fn dispatch_no_io(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();
    service.dispatch(cmd_id).send(&mut ipc_buf).map(|_| ())
}

/// CMIF request with a `Copy` input and no output.
#[inline]
pub(crate) fn dispatch_in<I>(service: &Session, cmd_id: u32, input: &I) -> Result<(), DispatchError>
where
    I: zerocopy::IntoBytes + zerocopy::Immutable,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// CMIF request with a `Copy` input, PID, and no output.
#[inline]
pub(crate) fn dispatch_in_pid<I>(
    service: &Session,
    cmd_id: u32,
    input: &I,
) -> Result<(), DispatchError>
where
    I: zerocopy::IntoBytes + zerocopy::Immutable,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .send_pid()
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// CMIF request with a `Copy` input and a `Copy` output.
#[inline]
pub(crate) fn dispatch_in_out<I, O>(
    service: &Session,
    cmd_id: u32,
    input: &I,
) -> Result<O, DispatchError>
where
    I: zerocopy::IntoBytes + zerocopy::Immutable,
    O: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .out_size(size_of::<O>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<O>())
}
