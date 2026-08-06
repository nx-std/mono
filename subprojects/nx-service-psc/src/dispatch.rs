//! CMIF dispatch helpers shared across the `cmif` module.

use core::mem::size_of;

use nx_sf::service::{
    DispatchError,
    DomainObject,
};

/// CMIF request with no input payload and no output payload.
#[inline]
pub(crate) fn dispatch_no_io(object: &DomainObject<'_>, cmd_id: u32) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();
    object.dispatch(cmd_id).send(&mut ipc_buf).map(|_| ())
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
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send(&mut ipc_buf)
        .map(|_| ())
}
