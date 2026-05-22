//! CMIF dispatch helpers shared across the `cmif` module.

use core::{mem::size_of, ptr};

use nx_sf::service::{DispatchError, Session};

/// CMIF domain request with no input, no output.
#[inline]
pub(crate) fn dispatch_domain_no_io(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service.dispatch(cmd_id).send(&mut ipc_buf).map(|_| ())
}

/// CMIF domain request with a raw input payload, no output.
#[inline]
pub(crate) fn dispatch_domain_in_no_out<T>(
    service: &Session,
    cmd_id: u32,
    input: &T,
) -> Result<(), DispatchError> {
    // SAFETY: `input` is a valid reference; viewing its `size_of::<T>()` bytes
    // as a slice is sound, and the slice borrows `input`.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const *input).cast::<u8>(), size_of::<T>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// CMIF domain request with a raw input payload and raw output.
#[inline]
pub(crate) fn dispatch_domain_in_out<T, U: Copy>(
    service: &Session,
    cmd_id: u32,
    input: &T,
) -> Result<U, DispatchError> {
    // SAFETY: `input` is a valid reference; viewing its `size_of::<T>()` bytes
    // as a slice is sound, and the slice borrows `input`.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const *input).cast::<u8>(), size_of::<T>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<U>())
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<U>().
    let val = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<U>()) };

    Ok(val)
}

/// CMIF domain request with no input, raw output.
#[inline]
pub(crate) fn dispatch_domain_out<U: Copy>(
    service: &Session,
    cmd_id: u32,
) -> Result<U, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<U>())
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<U>().
    let val = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<U>()) };

    Ok(val)
}
