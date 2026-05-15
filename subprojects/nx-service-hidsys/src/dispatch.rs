//! CMIF dispatch helpers shared across the `cmif` module.

use core::{mem::size_of, ptr};

use nx_sf::service::{DispatchError, Session};

/// CMIF request with no input payload and no output payload.
#[inline]
pub(crate) fn dispatch_no_io(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    service.dispatch(cmd_id).send().map(|_| ())
}

/// CMIF request with a `Copy` input and no output.
#[inline]
pub(crate) fn dispatch_in<I: Copy>(
    service: &Session,
    cmd_id: u32,
    input: &I,
) -> Result<(), DispatchError> {
    // SAFETY: `input` is a valid `Copy` value; viewing its `size_of::<I>()`
    // bytes as a slice is sound, and the slice lives until `.send()` returns.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((input as *const I).cast::<u8>(), size_of::<I>()) };
    service.dispatch(cmd_id).in_raw(in_bytes).send().map(|_| ())
}

/// CMIF request with no input and a single `Copy` output.
#[inline]
pub(crate) fn dispatch_out<O: Copy>(service: &Session, cmd_id: u32) -> Result<O, DispatchError> {
    let result = service.dispatch(cmd_id).out_size(size_of::<O>()).send()?;
    // SAFETY: the response payload is at least `size_of::<O>()` bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
}

/// CMIF request with a `Copy` input and a `Copy` output.
#[inline]
pub(crate) fn dispatch_in_out<I: Copy, O: Copy>(
    service: &Session,
    cmd_id: u32,
    input: &I,
) -> Result<O, DispatchError> {
    // SAFETY: `input` is a valid `Copy` value; viewing its `size_of::<I>()`
    // bytes as a slice is sound, and the slice lives until `.send()` returns.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((input as *const I).cast::<u8>(), size_of::<I>()) };
    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<O>())
        .send()?;

    // SAFETY: response payload is at least size_of::<O>() bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
}
