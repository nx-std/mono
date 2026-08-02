//! TIPC response parsing.

use nx_svc::{error::ResultCode, raw::Handle as RawHandle};
use nx_sys_thread_tls::IPC_BUFFER_SIZE;
use zerocopy::little_endian::U32;

use crate::{
    cursor::{Cursor, ResponsePayload},
    error::{GENERIC_ERROR, ToResultCode},
    hipc,
};

/// Parses a TIPC response.
///
/// Generic over `P: ResponsePayload`: pick the payload shape via
/// turbofish — `&T` for a zerocopy struct or `()` for responses that
/// carry no payload.
///
/// The first data word of a TIPC response is the result code; non-zero
/// codes surface as [`ParseResponseError::ServiceError`] before the
/// payload is parsed.
pub fn parse_response<'a, P>(
    buf: &'a [u8; IPC_BUFFER_SIZE],
) -> Result<Response<'a, P>, ParseResponseError>
where
    P: ResponsePayload<'a>,
{
    let envelope = hipc::parse_response_envelope(buf)?;
    let cursor = Cursor::new(envelope.data_words);

    let (result_word, cursor) = cursor
        .read::<U32>()
        .ok_or(ParseResponseError::TruncatedResult)?;
    let result = result_word.get();
    if result != 0 {
        return Err(ParseResponseError::ServiceError(result));
    }

    let (payload, _) = P::read(cursor).ok_or(ParseResponseError::TruncatedPayload)?;

    Ok(Response {
        payload,
        copy_handles: envelope.copy_handles,
        move_handles: envelope.move_handles,
    })
}

/// Error returned by [`parse_response`].
#[derive(Debug, thiserror::Error)]
pub enum ParseResponseError {
    /// Service returned a non-zero result code.
    #[error("service error: {0:#x}")]
    ServiceError(u32),
    /// Underlying HIPC layer rejected the response.
    #[error("HIPC parse: {0}")]
    Hipc(#[from] hipc::ResponseParseError),
    /// Response data words too small to contain the result-code word.
    #[error("TIPC response too small for result code")]
    TruncatedResult,
    /// Response too small to contain the caller-requested payload.
    #[error("TIPC response too small for payload")]
    TruncatedPayload,
}

impl ToResultCode for ParseResponseError {
    fn to_rc(self) -> ResultCode {
        match self {
            // The only variant carrying a code the server chose; every other
            // one is a shape this crate rejected after a successful reply.
            ParseResponseError::ServiceError(code) => code,
            ParseResponseError::Hipc(err) => err.to_rc(),
            ParseResponseError::TruncatedResult | ParseResponseError::TruncatedPayload => {
                GENERIC_ERROR
            }
        }
    }
}

/// Parsed TIPC response with a typed payload.
#[derive(Debug)]
pub struct Response<'a, P> {
    /// In-band response payload, in whatever shape the caller selected.
    pub payload: P,
    /// Returned copy handles.
    pub copy_handles: &'a [RawHandle],
    /// Returned move handles (used for receiving service objects).
    pub move_handles: &'a [RawHandle],
}
