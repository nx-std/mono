//! CMIF dispatch helpers shared across the `cmif` module.

use core::{
    mem::size_of,
    ptr,
};

use nx_sf::service::{
    DispatchError,
    Session,
};

/// CMIF request with a raw input payload, PID, and no output payload.
#[inline]
pub(crate) fn dispatch_in_pid_no_out<T>(
    service: &Session,
    cmd_id: u32,
    input: &T,
) -> Result<(), DispatchError> {
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<T>()` bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const *input).cast::<u8>(), size_of::<T>()) };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send_pid()
        .send(&mut buf)
        .map(|_| ())
}

/// CMIF request with a raw input payload, PID, and a u64 output.
#[inline]
pub(crate) fn dispatch_in_pid_out_u64<T>(
    service: &Session,
    cmd_id: u32,
    input: &T,
) -> Result<u64, DispatchError> {
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<T>()` bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const *input).cast::<u8>(), size_of::<T>()) };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send_pid()
        .out_size(size_of::<u64>())
        .send(&mut buf)?;

    // SAFETY: response payload is at least size_of::<u64>().
    let val = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };

    Ok(val)
}

/// CMIF request with a u64 input and no output payload.
#[inline]
pub(crate) fn dispatch_in_u64_no_out(
    service: &Session,
    cmd_id: u32,
    value: u64,
) -> Result<(), DispatchError> {
    // SAFETY: `value` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const value).cast::<u8>(), size_of::<u64>()) };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send(&mut buf)
        .map(|_| ())
}

/// CMIF request with a u64 input and a u64 output.
#[inline]
pub(crate) fn dispatch_in_u64_out_u64(
    service: &Session,
    cmd_id: u32,
    value: u64,
) -> Result<u64, DispatchError> {
    // SAFETY: `value` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const value).cast::<u8>(), size_of::<u64>()) };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<u64>())
        .send(&mut buf)?;

    // SAFETY: response payload is at least size_of::<u64>().
    let val = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };

    Ok(val)
}
