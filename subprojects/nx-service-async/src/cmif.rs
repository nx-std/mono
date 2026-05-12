//! CMIF dispatch operations for IAsyncValue and IAsyncResult sub-objects.

use core::{mem::size_of, ptr};

use nx_sf::service::{BufferAttr, DispatchError, Session};

use crate::{proto, types::ErrorContext};

// ---------------------------------------------------------------------------
// IAsyncValue commands
// ---------------------------------------------------------------------------

/// Queries the value size (IAsyncValue cmd 0).
pub fn async_value_get_size(service: &Session) -> Result<u64, DispatchError> {
    let result = service
        .dispatch(proto::ASYNC_VALUE_GET_SIZE)
        .out_size(size_of::<u64>())
        .send()?;

    // SAFETY: response payload is at least size_of::<u64>() bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) })
}

/// Retrieves the value into a caller-supplied buffer (IAsyncValue cmd 1).
pub fn async_value_get(service: &Session, buffer: &mut [u8]) -> Result<(), DispatchError> {
    service
        .dispatch(proto::ASYNC_VALUE_GET)
        .buffer(
            buffer.as_mut_ptr(),
            buffer.len(),
            BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send()
        .map(|_| ())
}

/// Cancels the async operation (IAsyncValue cmd 2).
pub fn async_value_cancel(service: &Session) -> Result<(), DispatchError> {
    service
        .dispatch(proto::ASYNC_VALUE_CANCEL)
        .send()
        .map(|_| ())
}

/// Retrieves the error context (IAsyncValue cmd 3, `[4.0.0+]`).
pub fn async_value_get_error_context(
    service: &Session,
    context: &mut ErrorContext,
) -> Result<(), DispatchError> {
    service
        .dispatch(proto::ASYNC_VALUE_GET_ERROR_CONTEXT)
        .buffer(
            (context as *mut ErrorContext).cast::<u8>(),
            size_of::<ErrorContext>(),
            BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send()
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// IAsyncResult commands
// ---------------------------------------------------------------------------

/// Retrieves the result code (IAsyncResult cmd 0).
pub fn async_result_get(service: &Session) -> Result<(), DispatchError> {
    service.dispatch(proto::ASYNC_RESULT_GET).send().map(|_| ())
}

/// Cancels the async operation (IAsyncResult cmd 1).
pub fn async_result_cancel(service: &Session) -> Result<(), DispatchError> {
    service
        .dispatch(proto::ASYNC_RESULT_CANCEL)
        .send()
        .map(|_| ())
}

/// Retrieves the error context (IAsyncResult cmd 2, `[4.0.0+]`).
pub fn async_result_get_error_context(
    service: &Session,
    context: &mut ErrorContext,
) -> Result<(), DispatchError> {
    service
        .dispatch(proto::ASYNC_RESULT_GET_ERROR_CONTEXT)
        .buffer(
            (context as *mut ErrorContext).cast::<u8>(),
            size_of::<ErrorContext>(),
            BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send()
        .map(|_| ())
}
