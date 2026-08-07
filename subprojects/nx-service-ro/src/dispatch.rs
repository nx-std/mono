//! CMIF dispatch helpers shared across the `cmif` module.

use nx_sf::service::{
    DispatchError,
    Session,
};

/// CMIF request with a single `Copy` input payload, PID, and no output.
#[inline]
pub(crate) fn dispatch_in_pid<I>(
    service: &Session,
    cmd_id: u32,
    input: I,
) -> Result<(), DispatchError>
where
    I: zerocopy::IntoBytes + zerocopy::Immutable,
{
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .send_pid()
        .send(&mut buf)
        .map(|_| ())
}
