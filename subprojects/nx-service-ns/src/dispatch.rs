//! CMIF dispatch helpers shared across the `cmif` module.

use core::{mem::size_of, ptr};

use nx_sf::service::{DispatchError, Session};

#[inline]
pub(crate) fn dispatch_no_io(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    service.dispatch(cmd_id).send().map(|_| ())
}

#[inline]
pub(crate) fn dispatch_out<O: Copy>(service: &Session, cmd_id: u32) -> Result<O, DispatchError> {
    let result = service.dispatch(cmd_id).out_size(size_of::<O>()).send()?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
}

#[inline]
pub(crate) fn dispatch_in<I: Copy>(
    service: &Session,
    cmd_id: u32,
    input: I,
) -> Result<(), DispatchError> {
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<I>()) };
    service.dispatch(cmd_id).in_raw(in_bytes).send().map(|_| ())
}

#[inline]
pub(crate) fn dispatch_in_out<I: Copy, O: Copy>(
    service: &Session,
    cmd_id: u32,
    input: I,
) -> Result<O, DispatchError> {
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<I>()) };
    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<O>())
        .send()?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
}
