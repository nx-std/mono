//! CMIF dispatch helpers shared across the `cmif` module.

use core::mem::size_of;

use nx_sf::service::{
    DispatchError,
    Session,
};
use zerocopy::IntoBytes as _;

/// CMIF request with a raw input payload and no output payload.
#[inline]
pub(crate) fn dispatch_in_no_out<T>(
    service: &Session,
    cmd_id: u32,
    input: &T,
) -> Result<(), DispatchError>
where
    T: zerocopy::IntoBytes + zerocopy::Immutable,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// CMIF request with a raw input payload and a raw output payload.
#[inline]
pub(crate) fn dispatch_in_out<I, O>(
    service: &Session,
    cmd_id: u32,
    input: &I,
) -> Result<O, DispatchError>
where
    I: zerocopy::IntoBytes + zerocopy::Immutable,
    O: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .out_size(size_of::<O>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<O>())
}

/// CMIF request with a raw input payload, PID, and no output payload.
#[inline]
pub(crate) fn dispatch_in_pid_no_out<T>(
    service: &Session,
    cmd_id: u32,
    input: &T,
) -> Result<(), DispatchError>
where
    T: zerocopy::IntoBytes + zerocopy::Immutable,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .send_pid()
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// CMIF request with a u64 input and no output payload.
#[inline]
pub(crate) fn dispatch_in_u64_no_out(
    service: &Session,
    cmd_id: u32,
    value: u64,
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(value.as_bytes())
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// CMIF request with a u64 input and a u64 output.
#[inline]
pub(crate) fn dispatch_in_u64_out_u64(
    service: &Session,
    cmd_id: u32,
    value: u64,
) -> Result<u64, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(value.as_bytes())
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<u64>())
}
