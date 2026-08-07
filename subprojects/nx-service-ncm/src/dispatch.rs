//! CMIF dispatch helpers shared across the `cmif` module.

use core::{
    mem::size_of,
    ptr,
};

use nx_sf::service::{
    DispatchError,
    Session,
};

/// CMIF request with a single `Copy` input and a single `Copy` output.
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
    // SAFETY: the response payload is at least `size_of::<O>()` bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
}

/// CMIF request with no input and a single `Copy` output.
#[inline]
pub(crate) fn dispatch_out<O: Copy>(service: &Session, cmd_id: u32) -> Result<O, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<O>())
        .send(&mut ipc_buf)?;
    // SAFETY: the response payload is at least `size_of::<O>()` bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
}

/// CMIF request with a single `Copy` input and no output.
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
        .send(&mut ipc_buf)?;
    Ok(())
}

/// CMIF request with no input and no output.
#[inline]
pub(crate) fn dispatch_no_io(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service.dispatch(cmd_id).send(&mut ipc_buf)?;
    Ok(())
}

/// CMIF request with a `u8` input that returns a move handle (sub-object).
#[inline]
pub(crate) fn dispatch_in_u8_out_object(
    service: &Session,
    cmd_id: u32,
    input: u8,
) -> Result<u32, OpenSubObjectError> {
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<u8>()` byte as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<u8>()) };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send(&mut ipc_buf)
        .map_err(OpenSubObjectError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(OpenSubObjectError::MissingHandle);
    }
    Ok(result.move_handles[0])
}

/// Errors for commands that return a sub-object move handle.
#[derive(Debug, thiserror::Error)]
pub enum OpenSubObjectError {
    #[error("IPC dispatch failed")]
    Dispatch(#[source] DispatchError),
    #[error("expected move handle in response but none received")]
    MissingHandle,
}
