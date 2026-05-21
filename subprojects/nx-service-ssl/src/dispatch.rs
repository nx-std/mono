//! CMIF dispatch helpers shared across the `cmif` module.

use core::mem::size_of;

use nx_sf::service::{DispatchError, DomainObject};

/// CMIF request with no input payload and no output payload.
#[inline]
pub(crate) fn dispatch_no_io(object: &DomainObject<'_>, cmd_id: u32) -> Result<(), DispatchError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    object.dispatch(cmd_id).send(&mut buf).map(|_| ())
}

/// CMIF request with a single `Copy` input payload and no output.
#[inline]
pub(crate) fn dispatch_in<I: Copy>(
    object: &DomainObject<'_>,
    cmd_id: u32,
    input: I,
) -> Result<(), DispatchError> {
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<I>()` bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<I>()) };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    object
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send(&mut buf)
        .map(|_| ())
}

/// CMIF request with a single `Copy` input payload and a `u32` output.
#[inline]
pub(crate) fn dispatch_in_out_u32<I: Copy>(
    object: &DomainObject<'_>,
    cmd_id: u32,
    input: I,
) -> Result<u32, DispatchError> {
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<I>()` bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<I>()) };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = object
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<u32>())
        .send(&mut buf)?;
    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// CMIF request with no input and a `u32` output.
#[inline]
pub(crate) fn dispatch_out_u32(
    object: &DomainObject<'_>,
    cmd_id: u32,
) -> Result<u32, DispatchError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = object
        .dispatch(cmd_id)
        .out_size(size_of::<u32>())
        .send(&mut buf)?;
    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}
