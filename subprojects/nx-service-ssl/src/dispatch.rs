//! CMIF dispatch helpers shared across the `cmif` module.

use core::mem::size_of;

use nx_sf::service::{
    DispatchError,
    DomainObjectRef,
};

/// CMIF request with no input payload and no output payload.
#[inline]
pub(crate) fn dispatch_no_io(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();
    object.dispatch(cmd_id).send(&mut buf).map(|_| ())
}

/// CMIF request with a single `Copy` input payload and no output.
#[inline]
pub(crate) fn dispatch_in<I>(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    input: I,
) -> Result<(), DispatchError>
where
    I: zerocopy::IntoBytes + zerocopy::Immutable,
{
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .send(&mut buf)
        .map(|_| ())
}

/// CMIF request with a single `Copy` input payload and a `u32` output.
#[inline]
pub(crate) fn dispatch_in_out_u32<I>(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    input: I,
) -> Result<u32, DispatchError>
where
    I: zerocopy::IntoBytes + zerocopy::Immutable,
{
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .out_size(size_of::<u32>())
        .send(&mut buf)?;
    Ok(*result.value::<u32>())
}

/// CMIF request with no input and a `u32` output.
#[inline]
pub(crate) fn dispatch_out_u32(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
) -> Result<u32, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(cmd_id)
        .out_size(size_of::<u32>())
        .send(&mut buf)?;
    Ok(*result.value::<u32>())
}
