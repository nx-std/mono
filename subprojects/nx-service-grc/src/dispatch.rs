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

/// CMIF request with a `u64` input and no output payload.
#[inline]
pub(crate) fn dispatch_in_u64(
    service: &Session,
    cmd_id: u32,
    value: u64,
) -> Result<(), DispatchError> {
    // SAFETY: `value` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<u64>()` bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const value).cast::<u8>(), size_of::<u64>()) };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// CMIF request with a `u64` input and a `u32` output.
#[inline]
pub(crate) fn dispatch_in_u64_out_u32(
    service: &Session,
    cmd_id: u32,
    value: u64,
) -> Result<u32, DispatchError> {
    // SAFETY: `value` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<u64>()` bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const value).cast::<u8>(), size_of::<u64>()) };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<u32>())
        .send(&mut ipc_buf)?;
    // SAFETY: response payload is at least size_of::<u32>() bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}
