//! Small CMIF dispatch helpers shared between the `cmif/*` sub-service modules.
//!
//! Each helper wraps the [`DomainObject::dispatch`] builder for a single shape
//! (no-io / pod-in / pod-out). Variants with buffer attrs, `send_pid`, or
//! out-handles live inline next to their callers.

use core::{
    mem::size_of,
    ptr,
};

use nx_sf::service::{
    DispatchError,
    DomainObjectRef,
};

/// CMIF request with no input payload and no output payload.
#[inline]
pub(crate) fn dispatch_no_io(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object.dispatch(cmd_id).send(&mut ipc_buf).map(|_| ())
}

/// CMIF request with a single `Copy` input payload and no output.
#[inline]
pub(crate) fn dispatch_in<I: Copy>(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    input: I,
) -> Result<(), DispatchError> {
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<I>()` bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<I>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// CMIF request with no input and a single `Copy` output payload.
#[inline]
pub(crate) fn dispatch_out<O: Copy>(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
) -> Result<O, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(cmd_id)
        .out_size(size_of::<O>())
        .send(&mut ipc_buf)?;
    // SAFETY: the response payload is at least `size_of::<O>()` bytes — CMIF
    // `parse_response` would have errored otherwise.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
}
