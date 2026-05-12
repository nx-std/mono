//! Tiny CMIF dispatch helpers shared between the `cmif/*` sub-service modules.
//!
//! Each helper wraps the `nx_sf::service` dispatch builder for a single shape
//! (no-io / pod-in / pod-out / pod-in-pod-out). Per-method variants that need
//! buffer attrs, `send_pid`, or out-handles live inline in the sub-service
//! modules.

use core::{mem::size_of, ptr};

use nx_sf::service::{DispatchError, Session};

/// Trait abstracting over the dispatch entry points the LCS / ICPM helpers
/// use. Implemented for `&Session` (non-domain monitor service) and
/// `&DomainObject<'_>` (domain sub-objects).
pub(crate) trait DispatchTarget {
    fn dispatch(&self, request_id: u32) -> nx_sf::service::Dispatch<'_>;
}

impl DispatchTarget for Session {
    #[inline]
    fn dispatch(&self, request_id: u32) -> nx_sf::service::Dispatch<'_> {
        Session::dispatch(self, request_id)
    }
}

impl DispatchTarget for nx_sf::service::DomainObject<'_> {
    #[inline]
    fn dispatch(&self, request_id: u32) -> nx_sf::service::Dispatch<'_> {
        nx_sf::service::DomainObject::dispatch(self, request_id)
    }
}

/// Sends a CMIF request with no input payload and no output payload.
#[inline]
pub(crate) fn dispatch_no_io(
    target: &(impl DispatchTarget + ?Sized),
    cmd_id: u32,
) -> Result<(), DispatchError> {
    target.dispatch(cmd_id).send().map(|_| ())
}

/// Sends a CMIF request with a single `Copy` input payload and no output.
#[inline]
pub(crate) fn dispatch_in<I: Copy>(
    target: &(impl DispatchTarget + ?Sized),
    cmd_id: u32,
    input: I,
) -> Result<(), DispatchError> {
    // SAFETY: `input` lives on the stack until the `.send()` call returns
    // because Rust drops it at the end of this function. The dispatcher
    // memcpys the bytes out before sending.
    unsafe {
        target
            .dispatch(cmd_id)
            .in_raw((&raw const input).cast::<u8>(), size_of::<I>())
            .send()
            .map(|_| ())
    }
}

/// Sends a CMIF request with no input and reads a single `Copy` output payload.
#[inline]
pub(crate) fn dispatch_out<O: Copy>(
    target: &(impl DispatchTarget + ?Sized),
    cmd_id: u32,
) -> Result<O, DispatchError> {
    let result = target.dispatch(cmd_id).out_size(size_of::<O>()).send()?;
    // SAFETY: response payload is at least `size_of::<O>()` bytes by virtue of
    // `out_size`; CMIF parse_response would have errored otherwise.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
}
