//! CMIF dispatch helpers shared by the interfaces this crate serves.
//!
//! Most settings commands take nothing and answer with one scalar, so the request they build
//! differs only in the command id and the width of the answer. That shape lives here once.

use nx_sf::service::{
    DispatchError,
    Session,
};

/// CMIF request with no input and a single `Copy` output.
///
/// `T` is decoded from the response bytes, so it must accept every bit pattern the interface can
/// answer with. A type whose validity depends on its value, an enum over a fixed set of values
/// say, does not qualify: decode its wire form here and validate it into the domain type at the
/// call site.
///
/// # Errors
///
/// Returns [`DispatchError`] when the request could not be sent or the reply could not be
/// decoded. Nothing is answered.
#[inline]
pub(crate) fn dispatch_out<T>(session: &Session, cmd_id: u32) -> Result<T, DispatchError>
where
    T: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = session
        .dispatch(cmd_id)
        .out_size(size_of::<T>())
        .send(&mut buf)?;

    Ok(*result.value::<T>())
}

/// CMIF request with a single `Copy` input and a single `Copy` output.
///
/// `O` carries the same constraint as the output of [`dispatch_out`].
///
/// # Errors
///
/// The same as [`dispatch_out`].
#[inline]
pub(crate) fn dispatch_in_out<I, O>(
    session: &Session,
    cmd_id: u32,
    input: I,
) -> Result<O, DispatchError>
where
    I: zerocopy::IntoBytes + zerocopy::Immutable,
    O: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = session
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .out_size(size_of::<O>())
        .send(&mut buf)?;

    Ok(*result.value::<O>())
}
