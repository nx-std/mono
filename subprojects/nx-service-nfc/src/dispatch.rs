//! CMIF dispatch helpers shared across the `cmif` module.

use core::mem::size_of;

use nx_sf::service::{DispatchError, DomainObject};

/// CMIF request with no input payload and no output payload.
#[inline]
pub(crate) fn dispatch_no_io(object: &DomainObject<'_>, cmd_id: u32) -> Result<(), DispatchError> {
    object.dispatch(cmd_id).send().map(|_| ())
}

/// CMIF request with a single `Copy` input payload and no output.
#[inline]
pub(crate) fn dispatch_in<I: Copy>(
    object: &DomainObject<'_>,
    cmd_id: u32,
    input: I,
) -> Result<(), DispatchError> {
    // SAFETY: `input` lives on the stack until `.send()` returns; the dispatcher
    // memcpys the bytes into the IPC buffer before sending.
    unsafe {
        object
            .dispatch(cmd_id)
            .in_raw((&raw const input).cast::<u8>(), size_of::<I>())
            .send()
            .map(|_| ())
    }
}

/// CMIF request with a single `Copy` input payload and a `Copy` output.
#[inline]
pub(crate) fn dispatch_in_out<I: Copy, O: Copy>(
    object: &DomainObject<'_>,
    cmd_id: u32,
    input: I,
) -> Result<O, DispatchError> {
    // SAFETY: `input` lives on the stack until `.send()` returns.
    let result = unsafe {
        object
            .dispatch(cmd_id)
            .in_raw((&raw const input).cast::<u8>(), size_of::<I>())
            .out_size(size_of::<O>())
            .send()?
    };
    // SAFETY: the response payload is at least `size_of::<O>()` bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
}

/// CMIF request with no input and a `Copy` output.
#[inline]
pub(crate) fn dispatch_out<O: Copy>(
    object: &DomainObject<'_>,
    cmd_id: u32,
) -> Result<O, DispatchError> {
    let result = object.dispatch(cmd_id).out_size(size_of::<O>()).send()?;
    // SAFETY: the response payload is at least `size_of::<O>()` bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
}
