//! CMIF dispatch helpers shared across the `cmif` module.

use core::mem::size_of;

use nx_sf::service::{DispatchError, Session};

/// CMIF request with a single `Copy` input payload, PID, and no output.
#[inline]
pub(crate) fn dispatch_in_pid<I: Copy>(
    service: &Session,
    cmd_id: u32,
    input: I,
) -> Result<(), DispatchError> {
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<I>()` bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<I>()) };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send_pid()
        .send(&mut buf)
        .map(|_| ())
}
