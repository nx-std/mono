//! Tiny CMIF dispatch helpers shared between the `cmif/*` sub-service modules.
//!
//! Each helper wraps the `nx_sf::service` dispatch builder for a single shape
//! (no-io / pod-in / pod-out / pod-in-pod-out). Per-method variants that need
//! buffer attrs, `send_pid`, or out-handles live inline in the sub-service
//! modules.

use core::{mem::size_of, ptr};

use nx_sf::service::{DispatchError, DomainObject, Session};

/// Trait abstracting over the dispatch entry points the LCS / ICPM helpers
/// use. Implemented for `&Session` (non-domain monitor service) and
/// `&DomainObject<'_>` (domain sub-objects).
///
/// Since `Session::dispatch` and `DomainObject::dispatch` return different
/// builder types (`Dispatch` vs `DomainDispatch`), the trait exposes
/// fully-configured CMIF operations rather than the builder itself.
pub(crate) trait DispatchTarget {
    /// Sends `cmd_id` with no input and no output payload.
    fn send_no_io(&self, cmd_id: u32) -> Result<(), DispatchError>;

    /// Sends `cmd_id` with `size` bytes of input read from `ptr` and no output.
    ///
    /// # Safety
    ///
    /// `ptr` must be valid for `size` bytes for the duration of this call.
    unsafe fn send_in_raw(
        &self,
        cmd_id: u32,
        ptr: *const u8,
        size: usize,
    ) -> Result<(), DispatchError>;

    /// Sends `cmd_id` with no input and reads a single `Copy` output payload.
    fn read_out<O: Copy>(&self, cmd_id: u32) -> Result<O, DispatchError>;
}

impl DispatchTarget for Session {
    #[inline]
    fn send_no_io(&self, cmd_id: u32) -> Result<(), DispatchError> {
        self.dispatch(cmd_id).send().map(|_| ())
    }

    #[inline]
    unsafe fn send_in_raw(
        &self,
        cmd_id: u32,
        ptr: *const u8,
        size: usize,
    ) -> Result<(), DispatchError> {
        // SAFETY: caller upholds the `ptr`/`size` contract.
        unsafe { self.dispatch(cmd_id).in_raw(ptr, size).send().map(|_| ()) }
    }

    #[inline]
    fn read_out<O: Copy>(&self, cmd_id: u32) -> Result<O, DispatchError> {
        let result = self.dispatch(cmd_id).out_size(size_of::<O>()).send()?;
        // SAFETY: response payload is at least `size_of::<O>()` bytes.
        Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
    }
}

impl DispatchTarget for DomainObject<'_> {
    #[inline]
    fn send_no_io(&self, cmd_id: u32) -> Result<(), DispatchError> {
        self.dispatch(cmd_id).send().map(|_| ())
    }

    #[inline]
    unsafe fn send_in_raw(
        &self,
        cmd_id: u32,
        ptr: *const u8,
        size: usize,
    ) -> Result<(), DispatchError> {
        // SAFETY: caller upholds the `ptr`/`size` contract.
        unsafe { self.dispatch(cmd_id).in_raw(ptr, size).send().map(|_| ()) }
    }

    #[inline]
    fn read_out<O: Copy>(&self, cmd_id: u32) -> Result<O, DispatchError> {
        let result = self.dispatch(cmd_id).out_size(size_of::<O>()).send()?;
        // SAFETY: response payload is at least `size_of::<O>()` bytes.
        Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
    }
}

/// Sends a CMIF request with no input payload and no output payload.
#[inline]
pub(crate) fn dispatch_no_io(
    target: &(impl DispatchTarget + ?Sized),
    cmd_id: u32,
) -> Result<(), DispatchError> {
    target.send_no_io(cmd_id)
}

/// Sends a CMIF request with a single `Copy` input payload and no output.
#[inline]
pub(crate) fn dispatch_in<I: Copy>(
    target: &(impl DispatchTarget + ?Sized),
    cmd_id: u32,
    input: I,
) -> Result<(), DispatchError> {
    // SAFETY: `input` lives on the stack until `send_in_raw` returns.
    unsafe { target.send_in_raw(cmd_id, (&raw const input).cast::<u8>(), size_of::<I>()) }
}

/// Sends a CMIF request with no input and reads a single `Copy` output payload.
#[inline]
pub(crate) fn dispatch_out<O: Copy>(
    target: &(impl DispatchTarget + ?Sized),
    cmd_id: u32,
) -> Result<O, DispatchError> {
    target.read_out(cmd_id)
}
