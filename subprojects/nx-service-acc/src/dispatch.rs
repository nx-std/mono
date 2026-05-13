//! CMIF dispatch helpers shared across the `cmif` module.

use core::{mem::size_of, ptr};

use nx_sf::service::{DispatchError, Service};

/// CMIF request with a single `Copy` input and a single `Copy` output.
#[inline]
pub(crate) fn dispatch_in_out<I: Copy, O: Copy>(
    service: &Service,
    cmd_id: u32,
    input: I,
) -> Result<O, DispatchError> {
    // SAFETY: `input` lives on the stack until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(cmd_id)
            .in_raw((&raw const input).cast::<u8>(), size_of::<I>())
            .out_size(size_of::<O>())
            .send()?
    };
    // SAFETY: the response payload is at least `size_of::<O>()` bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
}

/// CMIF request with no input and a single `Copy` output.
#[inline]
pub(crate) fn dispatch_out<O: Copy>(service: &Service, cmd_id: u32) -> Result<O, DispatchError> {
    let result = service.dispatch(cmd_id).out_size(size_of::<O>()).send()?;
    // SAFETY: the response payload is at least `size_of::<O>()` bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
}
