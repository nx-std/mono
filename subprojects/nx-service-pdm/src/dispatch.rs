//! CMIF dispatch helpers shared across the `cmif` module.

use core::mem::size_of;

use nx_sf::service::{
    DispatchError,
    Session,
};

/// CMIF request with a single `Copy` input and a single `Copy` output.
#[inline]
pub(crate) fn dispatch_in_out<I, O>(
    service: &Session,
    cmd_id: u32,
    input: I,
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

/// CMIF request with no input and a single `Copy` output.
#[inline]
pub(crate) fn dispatch_out<O>(service: &Session, cmd_id: u32) -> Result<O, DispatchError>
where
    O: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<O>())
        .send(&mut ipc_buf)?;
    Ok(*result.value::<O>())
}
