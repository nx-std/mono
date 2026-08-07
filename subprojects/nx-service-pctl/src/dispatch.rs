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

/// CMIF request with no input and a single `Copy` output.
#[inline]
pub(crate) fn dispatch_out<O>(object: DomainObjectRef<'_>, cmd_id: u32) -> Result<O, DispatchError>
where
    O: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(cmd_id)
        .out_size(size_of::<O>())
        .send(&mut buf)?;
    Ok(*result.value::<O>())
}
