//! CMIF dispatch helpers shared across the `cmif` module.

use nx_sf::service::{
    DispatchError,
    Domain,
};

/// CMIF request with a single `Copy` input and no output payload.
#[inline]
pub(crate) fn dispatch_in<I>(domain: &Domain, cmd_id: u32, input: &I) -> Result<(), DispatchError>
where
    I: zerocopy::IntoBytes + zerocopy::Immutable,
{
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .send(&mut buf)
        .map(|_| ())
}
