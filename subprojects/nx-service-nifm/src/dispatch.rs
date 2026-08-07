//! Small CMIF dispatch helpers shared between the `cmif/*` sub-service modules.
//!
//! Each helper wraps the [`DomainObject::dispatch`] builder for a single shape
//! (no-io / pod-in / pod-out). Variants with buffer attrs, `send_pid`, or
//! out-handles live inline next to their callers.

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
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// CMIF request with no input and a single `Copy` output payload.
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
