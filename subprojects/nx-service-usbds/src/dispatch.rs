//! CMIF dispatch helpers shared across the `cmif` module.

use core::mem::size_of;

use nx_sf::service::{
    DispatchError,
    Session,
};

/// CMIF domain request with no input, no output.
#[inline]
pub(crate) fn dispatch_domain_no_io(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();
    service.dispatch(cmd_id).send(&mut ipc_buf).map(|_| ())
}

/// CMIF domain request with a raw input payload, no output.
#[inline]
pub(crate) fn dispatch_domain_in_no_out<T>(
    service: &Session,
    cmd_id: u32,
    input: &T,
) -> Result<(), DispatchError>
where
    T: zerocopy::IntoBytes + zerocopy::Immutable,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// CMIF domain request with a raw input payload and raw output.
#[inline]
pub(crate) fn dispatch_domain_in_out<T, U>(
    service: &Session,
    cmd_id: u32,
    input: &T,
) -> Result<U, DispatchError>
where
    T: zerocopy::IntoBytes + zerocopy::Immutable,
    U: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .out_size(size_of::<U>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<U>())
}

/// CMIF domain request with no input, raw output.
#[inline]
pub(crate) fn dispatch_domain_out<U>(service: &Session, cmd_id: u32) -> Result<U, DispatchError>
where
    U: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<U>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<U>())
}
