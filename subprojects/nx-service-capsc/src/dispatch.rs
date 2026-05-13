//! CMIF dispatch helpers shared across the `cmif` module.

use core::{mem::size_of, ptr};

use nx_sf::service::{DispatchError, Session};

/// CMIF request with a raw input payload and no output payload.
#[inline]
pub(crate) fn dispatch_in_no_out<T>(
    service: &Session,
    cmd_id: u32,
    input: &T,
) -> Result<(), DispatchError> {
    // SAFETY: `input` lives on the stack until `.send()` returns.
    unsafe {
        service
            .dispatch(cmd_id)
            .in_raw((&raw const *input).cast::<u8>(), size_of::<T>())
            .send()
            .map(|_| ())
    }
}

/// CMIF request with a raw input payload and a raw output payload.
#[inline]
pub(crate) fn dispatch_in_out<I, O: Copy>(
    service: &Session,
    cmd_id: u32,
    input: &I,
) -> Result<O, DispatchError> {
    // SAFETY: `input` lives on the stack until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(cmd_id)
            .in_raw((&raw const *input).cast::<u8>(), size_of::<I>())
            .out_size(size_of::<O>())
            .send()?
    };

    // SAFETY: response payload is at least size_of::<O>().
    let val = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<O>()) };

    Ok(val)
}

/// CMIF request with a raw input payload, PID, and no output payload.
#[inline]
pub(crate) fn dispatch_in_pid_no_out<T>(
    service: &Session,
    cmd_id: u32,
    input: &T,
) -> Result<(), DispatchError> {
    // SAFETY: `input` lives on the stack until `.send()` returns.
    unsafe {
        service
            .dispatch(cmd_id)
            .in_raw((&raw const *input).cast::<u8>(), size_of::<T>())
            .send_pid()
            .send()
            .map(|_| ())
    }
}

/// CMIF request with a u64 input and no output payload.
#[inline]
pub(crate) fn dispatch_in_u64_no_out(
    service: &Session,
    cmd_id: u32,
    value: u64,
) -> Result<(), DispatchError> {
    // SAFETY: `value` lives on the stack until `.send()` returns.
    unsafe {
        service
            .dispatch(cmd_id)
            .in_raw((&raw const value).cast::<u8>(), size_of::<u64>())
            .send()
            .map(|_| ())
    }
}

/// CMIF request with a u64 input and a u64 output.
#[inline]
pub(crate) fn dispatch_in_u64_out_u64(
    service: &Session,
    cmd_id: u32,
    value: u64,
) -> Result<u64, DispatchError> {
    // SAFETY: `value` lives on the stack until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(cmd_id)
            .in_raw((&raw const value).cast::<u8>(), size_of::<u64>())
            .out_size(size_of::<u64>())
            .send()?
    };

    // SAFETY: response payload is at least size_of::<u64>().
    let val = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };

    Ok(val)
}
