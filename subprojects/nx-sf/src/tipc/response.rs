//! TIPC response parsing.

use core::mem::size_of;

use nx_svc::raw::Handle as RawHandle;
use nx_sys_thread_tls::IPC_BUFFER_SIZE;
use zerocopy::IntoBytes;

use crate::hipc;

/// Parses a TIPC response message.
pub fn parse_response<'a>(
    buf: &'a [u8; IPC_BUFFER_SIZE],
    size: usize,
) -> Result<Response<'a>, ParseResponseError> {
    let hipc_resp = hipc::parse_response(buf)?;

    // Result code is the first word of data.
    if hipc_resp.data_words.is_empty() {
        return Err(ParseResponseError::EmptyResponse);
    }

    let result = hipc_resp.data_words[0];
    if result != 0 {
        return Err(ParseResponseError::ServiceError(result));
    }

    // Skip the 4-byte result code prefix.
    let (_result_word, payload) = hipc_resp
        .data_words
        .as_bytes()
        .split_at_checked(size_of::<u32>())
        .ok_or(ParseResponseError::TruncatedResult)?;
    let (data, _) = payload
        .split_at_checked(size)
        .ok_or(ParseResponseError::TruncatedPayload)?;

    Ok(Response {
        data,
        copy_handles: hipc_resp.copy_handles,
        move_handles: hipc_resp.move_handles,
    })
}

/// Error returned by [`parse_response`].
#[derive(Debug, thiserror::Error)]
pub enum ParseResponseError {
    /// Response data words are empty.
    #[error("empty response data")]
    EmptyResponse,
    /// Service returned a non-zero result code.
    #[error("service error: {0:#x}")]
    ServiceError(u32),
    /// Underlying HIPC layer rejected the response.
    #[error("HIPC parse: {0}")]
    Hipc(#[from] hipc::ResponseParseError),
    /// Response data words too small to contain the result code word.
    #[error("TIPC response too small for result code")]
    TruncatedResult,
    /// Response too small to contain the caller-requested payload size.
    #[error("TIPC response too small for payload")]
    TruncatedPayload,
}

/// Parsed TIPC response.
#[derive(Debug)]
pub struct Response<'a> {
    /// Response payload data (excludes the result code word).
    pub data: &'a [u8],
    /// Returned copy handles.
    pub copy_handles: &'a [RawHandle],
    /// Returned move handles (used for receiving service objects).
    pub move_handles: &'a [RawHandle],
}
