//! CMIF dispatch helpers shared across the `cmif` module.

use core::mem::size_of;

use nx_sf::service::{DispatchError, Domain};

/// CMIF request with a single `Copy` input and no output payload.
#[inline]
pub(crate) fn dispatch_in<I: Copy>(
    domain: &Domain,
    cmd_id: u32,
    input: &I,
) -> Result<(), DispatchError> {
    // SAFETY: `input` lives on the caller's stack, valid until `.send()`
    // returns; viewing its `size_of::<I>()` bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((input as *const I).cast::<u8>(), size_of::<I>()) };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send(&mut buf)
        .map(|_| ())
}
