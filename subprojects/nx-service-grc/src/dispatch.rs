//! CMIF dispatch helpers shared across the `cmif` module.

use core::{mem::size_of, ptr};

use nx_sf::service::{DispatchError, Session};

/// CMIF request with no input payload and no output payload.
#[inline]
pub(crate) fn dispatch_no_io(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    service.dispatch(cmd_id).send().map(|_| ())
}

/// CMIF request with a `u64` input and no output payload.
#[inline]
pub(crate) fn dispatch_in_u64(
    service: &Session,
    cmd_id: u32,
    value: u64,
) -> Result<(), DispatchError> {
    unsafe {
        service
            .dispatch(cmd_id)
            .in_raw((&raw const value).cast::<u8>(), size_of::<u64>())
            .send()
            .map(|_| ())
    }
}

/// CMIF request with a `u64` input and a `u32` output.
#[inline]
pub(crate) fn dispatch_in_u64_out_u32(
    service: &Session,
    cmd_id: u32,
    value: u64,
) -> Result<u32, DispatchError> {
    let result = unsafe {
        service
            .dispatch(cmd_id)
            .in_raw((&raw const value).cast::<u8>(), size_of::<u64>())
            .out_size(size_of::<u32>())
            .send()?
    };
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}
