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
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();
    object.dispatch(cmd_id).send(&mut ipc_buf).map(|_| ())
}

/// CMIF request with a single input payload and no output.
#[inline]
pub(crate) fn dispatch_in<I>(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    input: I,
) -> Result<(), DispatchError>
where
    I: zerocopy::IntoBytes + zerocopy::Immutable,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// CMIF request with a single input payload and a single output payload.
#[inline]
pub(crate) fn dispatch_in_out<I, O>(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    input: I,
) -> Result<O, DispatchError>
where
    I: zerocopy::IntoBytes + zerocopy::Immutable,
    O: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .out_size(size_of::<O>())
        .send(&mut ipc_buf)?;
    Ok(*result.value::<O>())
}

/// CMIF request with no input and a single output payload.
#[inline]
pub(crate) fn dispatch_out<O>(object: DomainObjectRef<'_>, cmd_id: u32) -> Result<O, DispatchError>
where
    O: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(cmd_id)
        .out_size(size_of::<O>())
        .send(&mut ipc_buf)?;
    Ok(*result.value::<O>())
}
