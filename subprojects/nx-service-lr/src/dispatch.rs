//! CMIF dispatch helpers shared across the `cmif` module.

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Session,
};
use zerocopy::IntoBytes as _;

use crate::types::{
    LR_MAX_PATH,
    RedirectApplicationIn,
};

/// CMIF request with no input payload and no output payload.
#[inline]
pub(crate) fn dispatch_no_io(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();
    service.dispatch(cmd_id).send(&mut ipc_buf).map(|_| ())
}

/// Resolves a path: sends a `u64` title ID and receives a fixed-size path
/// buffer via HIPC pointer.
pub(crate) fn resolve_path(
    service: &Session,
    cmd_id: u32,
    tid: u64,
    out: &mut [u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(tid.as_bytes())
        .out_buffer(
            out.as_mut_slice(),
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Redirects a path: sends a `u64` title ID and an input path buffer via
/// HIPC pointer.
pub(crate) fn redirect_path(
    service: &Session,
    cmd_id: u32,
    tid: u64,
    path: &[u8; LR_MAX_PATH],
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(tid.as_bytes())
        .in_buffer(path.as_slice(), BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
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
    let input = RedirectApplicationIn { tid, tid2 };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .in_buffer(path.as_slice(), BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Dispatches a command with a single `u64` input and no output.
pub(crate) fn dispatch_in_u64(
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
