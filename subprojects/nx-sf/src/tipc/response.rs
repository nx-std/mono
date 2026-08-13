//! TIPC response messages: parsing one as a client, building one as a server.
//!
//! [`parse_response`] reads a reply a service sent; [`TipcReplyBuilder`]
//! produces one for a service this process hosts. Request building and parsing
//! live in the sibling `request` module.

use core::mem::size_of;

use nx_svc::{
    error::ResultCode,
    raw::Handle as RawHandle,
};
use nx_sys_thread_tls::IPC_BUFFER_SIZE;
use zerocopy::little_endian::U32;

use crate::{
    cursor::{
        Cursor,
        ResponsePayload,
    },
    error::{
        GENERIC_ERROR,
        ToResultCode,
    },
    hipc::{
        self,
        HipcPayload,
        HipcReply,
        HipcReplyBuilder,
        write_section,
    },
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

/// TIPC reply value.
///
/// Alias for a [`HipcReply`] carrying a [`TipcReplyBody`] payload. Serialize it
/// into the thread's IPC buffer with [`HipcReply::write_to`]; the server then
/// hands that buffer to the reply-and-receive syscall.
pub type TipcReply<P> = HipcReply<TipcReplyBody<P>>;

/// Fluent builder for a [`TipcReply`].
///
/// The result code is taken at construction rather than defaulted, because a
/// reply that forgot to report a failure is indistinguishable on the wire from
/// one that succeeded.
///
/// The payload is a [`HipcPayload`] rather than a byte slice, for the reason
/// [`CmifReplyBuilder`](crate::cmif::CmifReplyBuilder) gives.
///
/// Offers no send-static accumulator: TIPC has no pointer descriptors, so
/// returned data travels in the data words or in a mapped buffer the client
/// supplied.
pub struct TipcReplyBuilder<P: HipcPayload = ()> {
    hipc: HipcReplyBuilder,
    result: ResultCode,
    payload: P,
}

impl TipcReplyBuilder {
    /// Starts a builder for a reply reporting `result`, with no payload.
    ///
    /// Attach one via [`with_payload`](Self::with_payload).
    #[inline]
    pub fn new(result: ResultCode) -> Self {
        Self {
            hipc: HipcReplyBuilder::new(),
            result,
            payload: (),
        }
    }
}

impl<P: HipcPayload> TipcReplyBuilder<P> {
    /// Attaches the reply payload, type-changing the builder to carry `Q`.
    ///
    /// All previously-accumulated envelope state is preserved.
    #[inline]
    pub fn with_payload<Q: HipcPayload>(self, payload: Q) -> TipcReplyBuilder<Q> {
        TipcReplyBuilder {
            hipc: self.hipc,
            result: self.result,
            payload,
        }
    }

    /// Sets the reply payload to bytes copied in after the result code.
    #[inline]
    pub fn with_data(self, data: &[u8]) -> TipcReplyBuilder<&[u8]> {
        self.with_payload(data)
    }

    /// Sets the reply payload from a typed value via its zero-copy byte view.
    #[inline]
    pub fn with_data_value<T>(self, value: &T) -> TipcReplyBuilder<&[u8]>
    where
        T: zerocopy::IntoBytes + zerocopy::Immutable,
    {
        self.with_data(value.as_bytes())
    }

    /// Adds a copy handle, which the kernel duplicates into the client.
    ///
    /// # Panics
    ///
    /// Panics in debug builds once more than [`HIPC_MAX_DESCRIPTORS`] entries
    /// are added; the wire-format cap is hardware-fixed.
    ///
    /// [`HIPC_MAX_DESCRIPTORS`]: crate::hipc::HIPC_MAX_DESCRIPTORS
    #[inline]
    pub fn add_copy_handle(mut self, handle: RawHandle) -> Self {
        self.hipc = self.hipc.with_copy_handle(handle);
        self
    }

    /// Adds a move handle, transferring ownership to the client.
    ///
    /// TIPC returns service objects this way, where CMIF would return a domain
    /// object id.
    ///
    /// # Panics
    ///
    /// Panics in debug builds once more than [`HIPC_MAX_DESCRIPTORS`] entries
    /// are added; the wire-format cap is hardware-fixed.
    ///
    /// [`HIPC_MAX_DESCRIPTORS`]: crate::hipc::HIPC_MAX_DESCRIPTORS
    #[inline]
    pub fn add_move_handle(mut self, handle: RawHandle) -> Self {
        self.hipc = self.hipc.with_move_handle(handle);
        self
    }

    /// Finalizes the reply value.
    pub fn build(self) -> TipcReply<P> {
        let body = TipcReplyBody {
            result: self.result,
            payload: self.payload,
        };
        self.hipc.with_payload(body).build()
    }
}

/// In-band body for a TIPC reply.
///
/// Encodes the result code as the first data word, followed by the reply
/// payload. There is no magic header and no alignment pad: the whole of what
/// CMIF puts in an `OutHeader` is, here, one word.
#[derive(Debug, Clone)]
pub struct TipcReplyBody<P: HipcPayload> {
    result: ResultCode,
    payload: P,
}

impl<P: HipcPayload> HipcPayload for TipcReplyBody<P> {
    /// The result-code word plus the reply payload.
    fn encoded_len(&self) -> usize {
        size_of::<U32>() + self.payload.encoded_len()
    }

    /// Writes the result-code word, then the reply payload.
    fn write_to(&self, dst: &mut [u8]) {
        let buf = write_section(dst, &U32::new(self.result));
        self.payload.write_to(buf);
    }
}
