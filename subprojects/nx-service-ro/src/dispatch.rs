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
    // SAFETY: `input` lives on the stack until `.send()` returns.
    unsafe {
        service
            .dispatch(cmd_id)
            .in_raw((&raw const input).cast::<u8>(), size_of::<I>())
            .send_pid()
            .send()
            .map(|_| ())
    }
}
