//! CMIF dispatch operations for IAsyncValue and IAsyncResult sub-objects.

use core::{
    mem::size_of,
    ptr,
};

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Session,
};

use crate::{
    proto,
    types::ErrorContext,
};

// ---------------------------------------------------------------------------
// IAsyncValue commands
// ---------------------------------------------------------------------------

/// Queries the value size (IAsyncValue cmd 0).
pub fn async_value_get_size(service: &Session) -> Result<u64, DispatchError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::ASYNC_VALUE_GET_SIZE)
        .out_size(size_of::<u64>())
        .send(&mut buf)?;

    // SAFETY: response payload is at least size_of::<u64>() bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) })
}

/// Retrieves the value into a caller-supplied buffer (IAsyncValue cmd 1).
pub fn async_value_get(service: &Session, buffer: &mut [u8]) -> Result<(), DispatchError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::ASYNC_VALUE_GET)
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut buf)
        .map(|_| ())
}

/// Cancels the async operation (IAsyncValue cmd 2).
pub fn async_value_cancel(service: &Session) -> Result<(), DispatchError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::ASYNC_VALUE_CANCEL)
        .send(&mut buf)
        .map(|_| ())
}

/// Retrieves the error context (IAsyncValue cmd 3, `[4.0.0+]`).
pub fn async_value_get_error_context(
    service: &Session,
    context: &mut ErrorContext,
) -> Result<(), DispatchError> {
    // SAFETY: `ErrorContext` is `#[repr(C)]`; viewing its `size_of` bytes as a
    // byte slice for the OUT buffer is sound, and the slice borrows `context`.
    let bytes = unsafe {
        core::slice::from_raw_parts_mut(
            ptr::from_mut(context).cast::<u8>(),
            size_of::<ErrorContext>(),
        )
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::ASYNC_VALUE_GET_ERROR_CONTEXT)
        .out_buffer(bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut buf)
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// IAsyncResult commands
// ---------------------------------------------------------------------------

/// Retrieves the result code (IAsyncResult cmd 0).
pub fn async_result_get(service: &Session) -> Result<(), DispatchError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::ASYNC_RESULT_GET)
        .send(&mut buf)
        .map(|_| ())
}

/// Cancels the async operation (IAsyncResult cmd 1).
pub fn async_result_cancel(service: &Session) -> Result<(), DispatchError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::ASYNC_RESULT_CANCEL)
        .send(&mut buf)
        .map(|_| ())
}

/// Retrieves the error context (IAsyncResult cmd 2, `[4.0.0+]`).
pub fn async_result_get_error_context(
    service: &Session,
    context: &mut ErrorContext,
) -> Result<(), DispatchError> {
    // SAFETY: `ErrorContext` is `#[repr(C)]`; viewing its `size_of` bytes as a
    // byte slice for the OUT buffer is sound, and the slice borrows `context`.
    let bytes = unsafe {
        core::slice::from_raw_parts_mut(
            ptr::from_mut(context).cast::<u8>(),
            size_of::<ErrorContext>(),
        )
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::ASYNC_RESULT_GET_ERROR_CONTEXT)
        .out_buffer(bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut buf)
        .map(|_| ())
}
