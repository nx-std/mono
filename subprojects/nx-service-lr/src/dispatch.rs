//! CMIF dispatch helpers shared across the `cmif` module.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Session};

use crate::types::LR_MAX_PATH;

/// CMIF request with no input payload and no output payload.
#[inline]
pub(crate) fn dispatch_no_io(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    service.dispatch(cmd_id).send().map(|_| ())
}

/// Resolves a path: sends a `u64` title ID and receives a fixed-size path
/// buffer via HIPC pointer.
pub(crate) fn resolve_path(
    service: &Session,
    cmd_id: u32,
    tid: u64,
    out: &mut [u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    // SAFETY: `tid` lives on the stack until `.send()` returns.
    // `out` is a caller-provided buffer valid for the lifetime of this call.
    unsafe {
        service
            .dispatch(cmd_id)
            .in_raw((&raw const tid).cast::<u8>(), size_of::<u64>())
            .buffer(
                out.as_mut_ptr(),
                LR_MAX_PATH,
                BufferAttr::OUT
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::FIXED_SIZE),
            )
            .send()
            .map(|_| ())
    }
}

/// Redirects a path: sends a `u64` title ID and an input path buffer via
/// HIPC pointer.
pub(crate) fn redirect_path(
    service: &Session,
    cmd_id: u32,
    tid: u64,
    path: &[u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    // SAFETY: `tid` lives on the stack until `.send()` returns.
    // `path` is a caller-provided buffer valid for the lifetime of this call.
    unsafe {
        service
            .dispatch(cmd_id)
            .in_raw((&raw const tid).cast::<u8>(), size_of::<u64>())
            .buffer(
                path.as_ptr(),
                LR_MAX_PATH,
                BufferAttr::IN.or(BufferAttr::HIPC_POINTER),
            )
            .send()
            .map(|_| ())
    }
}

/// Redirects an application path (9.0.0+ variant): sends two `u64` title IDs
/// and an input path buffer via HIPC pointer.
pub(crate) fn redirect_application_path(
    service: &Session,
    cmd_id: u32,
    tid: u64,
    tid2: u64,
    path: &[u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    use crate::types::RedirectApplicationIn;

    let input = RedirectApplicationIn { tid, tid2 };

    // SAFETY: `input` lives on the stack until `.send()` returns.
    // `path` is a caller-provided buffer valid for the lifetime of this call.
    unsafe {
        service
            .dispatch(cmd_id)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<RedirectApplicationIn>(),
            )
            .buffer(
                path.as_ptr(),
                LR_MAX_PATH,
                BufferAttr::IN.or(BufferAttr::HIPC_POINTER),
            )
            .send()
            .map(|_| ())
    }
}

/// Dispatches a command with a single `u64` input and no output.
pub(crate) fn dispatch_in_u64(
    service: &Session,
    cmd_id: u32,
    value: u64,
) -> Result<(), DispatchError> {
    // SAFETY: `value` lives on the stack until `.send()` returns.
    unsafe {
        service
            .dispatch(cmd_id)
            .in_raw((&raw const value).cast::<u8>(), size_of::<u64>())
            .send()
            .map(|_| ())
    }
}
