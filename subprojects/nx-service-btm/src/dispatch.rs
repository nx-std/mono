//! CMIF dispatch helpers shared across the `cmif` module.

use core::{
    mem::size_of,
    ptr,
};

use nx_sf::service::{
    DispatchError,
    Session,
};

/// CMIF request with no input payload and no output payload.
#[inline]
pub(crate) fn dispatch_no_io(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();
    service.dispatch(cmd_id).send(&mut ipc_buf).map(|_| ())
}

/// CMIF request with no input, returns a `Copy` output value.
#[inline]
pub(crate) fn dispatch_out<O: Copy>(service: &Session, cmd_id: u32) -> Result<O, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<O>())
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<O>() bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
}

/// CMIF request with a `Copy` input and no output.
#[inline]
pub(crate) fn dispatch_in<I: Copy>(
    service: &Session,
    cmd_id: u32,
    input: I,
) -> Result<(), DispatchError> {
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<I>()` bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<I>()) };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// CMIF request with a `Copy` input and a `Copy` output.
#[inline]
pub(crate) fn dispatch_in_out<I: Copy, O: Copy>(
    service: &Session,
    cmd_id: u32,
    input: I,
) -> Result<O, DispatchError> {
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<I>()` bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<I>()) };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<O>())
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<O>() bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
}
