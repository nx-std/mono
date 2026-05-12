//! IAsyncValue / IAsyncResult command IDs.

/// IAsyncValue: returns the value size.
pub const ASYNC_VALUE_GET_SIZE: u32 = 0;

/// IAsyncValue: retrieves the value into a buffer.
pub const ASYNC_VALUE_GET: u32 = 1;

/// IAsyncValue: cancels the async operation.
pub const ASYNC_VALUE_CANCEL: u32 = 2;

/// IAsyncValue: retrieves the error context (`[4.0.0+]`).
pub const ASYNC_VALUE_GET_ERROR_CONTEXT: u32 = 3;

/// IAsyncResult: retrieves the result code.
pub const ASYNC_RESULT_GET: u32 = 0;

/// IAsyncResult: cancels the async operation.
pub const ASYNC_RESULT_CANCEL: u32 = 1;

/// IAsyncResult: retrieves the error context (`[4.0.0+]`).
pub const ASYNC_RESULT_GET_ERROR_CONTEXT: u32 = 2;
