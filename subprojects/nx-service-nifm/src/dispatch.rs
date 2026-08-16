//! Small CMIF dispatch helpers shared between the `cmif/*` sub-service modules.
//!
//! Each helper wraps the request builder for a single shape (no-io / pod-in / pod-out). Variants
//! with buffer attrs, `send_pid`, or out-handles live inline next to their callers.
//!
//! The bound is [`DomainTarget`] rather than a concrete object view, so one body serves both a
//! request this crate created and one a C caller owns. None of these shapes adopts an object the
//! reply carries, which is what makes the trait the right bound: a command that did would need
//! the domain itself.

use core::mem::size_of;

use nx_sf::service::{
    DispatchError,
    DomainTarget,
};

/// CMIF request with no input payload and no output payload.
#[inline]
pub(crate) fn dispatch_no_io<'d>(
    object: impl DomainTarget<'d>,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();
    object.request(cmd_id).send(&mut ipc_buf).map(|_| ())
}

/// CMIF request with a single `Copy` input payload and no output.
#[inline]
pub(crate) fn dispatch_in<'d, I>(
    object: impl DomainTarget<'d>,
    cmd_id: u32,
    input: I,
) -> Result<(), DispatchError>
where
    I: zerocopy::IntoBytes + zerocopy::Immutable,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .request(cmd_id)
        .in_raw(input.as_bytes())
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// CMIF request with no input and a single `Copy` output payload.
#[inline]
pub(crate) fn dispatch_out<'d, T>(
    object: impl DomainTarget<'d>,
    cmd_id: u32,
) -> Result<T, DispatchError>
where
    T: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(cmd_id)
        .out_size(size_of::<T>())
        .send(&mut ipc_buf)?;
    Ok(*result.value::<T>())
}
