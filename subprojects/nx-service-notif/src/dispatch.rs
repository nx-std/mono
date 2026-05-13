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
    // SAFETY: `input` lives on the caller's stack until `.send()` returns.
    unsafe {
        domain
            .dispatch(cmd_id)
            .in_raw((input as *const I).cast::<u8>(), size_of::<I>())
            .send()
            .map(|_| ())
    }
}
