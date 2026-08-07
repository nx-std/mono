//! CMIF dispatch helpers shared across the `cmif` module.

use core::mem::size_of;

use nx_sf::service::{
    DispatchError,
    Session,
};

/// CMIF request with a raw input payload + PID, no output.
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

/// CMIF request with a raw input payload (no PID), raw output.
#[inline]
pub(crate) fn dispatch_in_out<T, U>(
    service: &Session,
    cmd_id: u32,
    input: &T,
) -> Result<U, DispatchError>
where
    T: zerocopy::IntoBytes + zerocopy::Immutable,
    U: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .out_size(size_of::<U>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<U>())
}
