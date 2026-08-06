//! Tiny CMIF dispatch helpers shared between the `cmif/*` sub-service modules.
//!
//! Each helper wraps the `nx_sf::service` dispatch builder for a single shape
//! (no-io / pod-in / pod-out / pod-in-pod-out). Per-method variants that need
//! buffer attrs, `send_pid`, or out-handles live inline in the sub-service
//! modules.

use core::{
    mem::size_of,
    ptr,
};

use nx_sf::service::{
    DispatchError,
    DomainObjectRef,
    Session,
};

/// Trait abstracting over the dispatch entry points the LCS / ICPM helpers
/// use. Implemented for `&Session` (non-domain monitor service) and
/// `DomainObjectRef<'_>` (domain sub-objects).
///
/// Since `Session::dispatch` and `DomainObjectRef::dispatch` return different
/// builder types (`Dispatch` vs `DomainDispatch`), the trait exposes
/// fully-configured CMIF operations rather than the builder itself.
pub(crate) trait DispatchTarget {
    /// Sends `cmd_id` with no input and no output payload.
    fn send_no_io(&self, cmd_id: u32) -> Result<(), DispatchError>;

    /// Sends `cmd_id` with a `Copy` input payload and no output.
    fn send_in<I: Copy>(&self, cmd_id: u32, input: I) -> Result<(), DispatchError>;

    /// Sends `cmd_id` with no input and reads a single `Copy` output payload.
    fn read_out<O: Copy>(&self, cmd_id: u32) -> Result<O, DispatchError>;
}

impl DispatchTarget for Session {
    #[inline]
    fn send_no_io(&self, cmd_id: u32) -> Result<(), DispatchError> {
        let mut buf = nx_sys_thread_tls::ipc_buffer();
        self.dispatch(cmd_id).send(&mut buf).map(|_| ())
    }

    #[inline]
    fn send_in<I: Copy>(&self, cmd_id: u32, input: I) -> Result<(), DispatchError> {
        // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
        // returns; viewing its bytes as a slice is sound.
        let in_bytes =
            unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<I>()) };
        let mut buf = nx_sys_thread_tls::ipc_buffer();

        self.dispatch(cmd_id)
            .in_raw(in_bytes)
            .send(&mut buf)
            .map(|_| ())
    }

    #[inline]
    fn read_out<O: Copy>(&self, cmd_id: u32) -> Result<O, DispatchError> {
        let mut buf = nx_sys_thread_tls::ipc_buffer();

        let result = self
            .dispatch(cmd_id)
            .out_size(size_of::<O>())
            .send(&mut buf)?;
        // SAFETY: response payload is at least `size_of::<O>()` bytes.
        Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
    }
}

impl DispatchTarget for DomainObjectRef<'_> {
    #[inline]
    fn send_no_io(&self, cmd_id: u32) -> Result<(), DispatchError> {
        let mut buf = nx_sys_thread_tls::ipc_buffer();
        self.dispatch(cmd_id).send(&mut buf).map(|_| ())
    }

    #[inline]
    fn send_in<I: Copy>(&self, cmd_id: u32, input: I) -> Result<(), DispatchError> {
        // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
        // returns; viewing its bytes as a slice is sound.
        let in_bytes =
            unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<I>()) };
        let mut buf = nx_sys_thread_tls::ipc_buffer();

        self.dispatch(cmd_id)
            .in_raw(in_bytes)
            .send(&mut buf)
            .map(|_| ())
    }

    #[inline]
    fn read_out<O: Copy>(&self, cmd_id: u32) -> Result<O, DispatchError> {
        let mut buf = nx_sys_thread_tls::ipc_buffer();

        let result = self
            .dispatch(cmd_id)
            .out_size(size_of::<O>())
            .send(&mut buf)?;
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
    target.send_in(cmd_id, input)
}

/// Sends a CMIF request with no input and reads a single `Copy` output payload.
#[inline]
pub(crate) fn dispatch_out<O: Copy>(
    target: &(impl DispatchTarget + ?Sized),
    cmd_id: u32,
) -> Result<O, DispatchError> {
    target.read_out(cmd_id)
}
