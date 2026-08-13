//! CMIF response messages: parsing one as a client, building one as a server.
//!
//! [`parse_response`] and its variants read a reply a service sent;
//! [`CmifReplyBuilder`] produces one for a service this process hosts. Request
//! parsing and building live in the sibling `request` module.

use core::mem::size_of;

use nx_svc::{
    error::ResultCode,
    raw::Handle as RawHandle,
};
use nx_sys_thread_tls::IPC_BUFFER_SIZE;

use super::wire::{
    CMIF_HEADER_ALIGN,
    DomainOutHeader,
    OUT_HEADER_MAGIC,
    OutHeader,
};
use crate::{
    cursor::{
        Cursor,
        ResponsePayload,
    },
    error::{
        GENERIC_ERROR,
        LibnxError,
        ToResultCode,
        libnx_error,
    },
    hipc::{
        self,
        HipcPayload,
        HipcReply,
        HipcReplyBuilder,
        StaticDescriptor,
        write_section,
    },
};

/// Parses a CMIF non-domain response.
///
/// Generic over `P: ResponsePayload`: pick the payload shape via
/// turbofish — `&T` for a zerocopy struct or `()` for control responses
/// that carry no payload. For runtime-sized byte payloads (CMIF
/// `OutRawData`), use [`parse_response_bytes`] instead.
pub fn parse_response<'a, P>(buf: &'a [u8; IPC_BUFFER_SIZE]) -> Result<Response<'a, P>, ParseError>
where
    P: ResponsePayload<'a>,
{
    parse_response_with(buf, P::read)
}

/// Parses a CMIF non-domain response whose payload is a runtime-sized
/// byte region (CMIF `OutRawData`).
///
/// `payload_len` is the byte count taken from the response data words.
pub fn parse_response_bytes(
    buf: &[u8; IPC_BUFFER_SIZE],
    payload_len: usize,
) -> Result<Response<'_, &'_ [u8]>, ParseError> {
    parse_response_with(buf, |c| c.read_bytes(payload_len))
}

/// Parses a CMIF domain response.
///
/// Like [`parse_response`], but consumes the [`DomainOutHeader`] before
/// the standard `OutHeader` and exposes the trailing object-id table
/// on the returned [`Response::objects`].
pub fn parse_response_domain<'a, P>(
    buf: &'a [u8; IPC_BUFFER_SIZE],
) -> Result<Response<'a, P>, ParseError>
where
    P: ResponsePayload<'a>,
{
    parse_response_domain_with(buf, P::read)
}

/// Parses a CMIF domain response whose payload is a runtime-sized byte
/// region (CMIF `OutRawData`).
///
/// `payload_len` is the byte count taken from the response data words.
pub fn parse_response_domain_bytes(
    buf: &[u8; IPC_BUFFER_SIZE],
    payload_len: usize,
) -> Result<Response<'_, &'_ [u8]>, ParseError> {
    parse_response_domain_with(buf, |c| c.read_bytes(payload_len))
}

/// Shared non-domain parsing path; `read_payload` consumes the payload
/// section from the cursor and everything else is fixed.
#[inline]
fn parse_response_with<'a, P>(
    buf: &'a [u8; IPC_BUFFER_SIZE],
    read_payload: impl FnOnce(Cursor<'a>) -> Option<(P, Cursor<'a>)>,
) -> Result<Response<'a, P>, ParseError> {
    let envelope = hipc::parse_response_envelope(buf)?;
    let cursor = Cursor::new(envelope.data_words).align_to(CMIF_HEADER_ALIGN);

    let (out_hdr, cursor) = cursor
        .read::<OutHeader>()
        .ok_or(ParseError::TruncatedOutHeader)?;
    let (payload, _) = read_payload(cursor).ok_or(ParseError::TruncatedPayload)?;

    if out_hdr.magic != OUT_HEADER_MAGIC {
        return Err(ParseError::InvalidMagic);
    }
    if out_hdr.result != 0 {
        return Err(ParseError::ServiceError(out_hdr.result));
    }

    Ok(Response {
        payload,
        objects: &[],
        copy_handles: envelope.copy_handles,
        move_handles: envelope.move_handles,
    })
}

/// Shared domain parsing path; `read_payload` consumes the payload
/// section and the trailing object-id table is read afterward.
#[inline]
fn parse_response_domain_with<'a, P>(
    buf: &'a [u8; IPC_BUFFER_SIZE],
    read_payload: impl FnOnce(Cursor<'a>) -> Option<(P, Cursor<'a>)>,
) -> Result<Response<'a, P>, ParseError> {
    let envelope = hipc::parse_response_envelope(buf)?;
    let cursor = Cursor::new(envelope.data_words).align_to(CMIF_HEADER_ALIGN);

    let (domain_hdr, cursor) = cursor
        .read::<DomainOutHeader>()
        .ok_or(ParseError::TruncatedDomainHeader)?;
    let (out_hdr, cursor) = cursor
        .read::<OutHeader>()
        .ok_or(ParseError::TruncatedOutHeader)?;
    let (payload, cursor) = read_payload(cursor).ok_or(ParseError::TruncatedPayload)?;

    // Validated before the objects are read, matching libnx's `cmifParseResponse`:
    // an error reply need not carry the object area at all, and reading it first
    // reports a truncation where the server actually named a result.
    if out_hdr.magic != OUT_HEADER_MAGIC {
        return Err(ParseError::InvalidMagic);
    }
    if out_hdr.result != 0 {
        return Err(ParseError::ServiceError(out_hdr.result));
    }

    // A reply carrying no objects has no object area to read, and asking for a
    // zero-length `&[u32]` would still demand 4-byte alignment the payload need
    // not leave behind: libnx puts the objects at `header + payload size` with no
    // padding, so a one-byte payload lands them at an odd offset. Skipping the
    // read is what keeps every command whose payload is not word-sized parseable.
    //
    // TODO: a command returning objects *and* a non-word-sized payload still
    //  fails here. Supporting it means reading the ids byte-wise, the way libnx
    //  does, rather than borrowing them as an aligned slice.
    let objects: &[u32] = if domain_hdr.num_out_objects == 0 {
        &[]
    } else {
        let (objects, _) = cursor
            .read_slice::<u32>(domain_hdr.num_out_objects as usize)
            .ok_or(ParseError::TruncatedDomainObjects)?;
        objects
    };

    Ok(Response {
        payload,
        objects,
        copy_handles: envelope.copy_handles,
        move_handles: envelope.move_handles,
    })
}

/// Parsed CMIF response with a typed payload.
#[derive(Debug)]
pub struct Response<'a, P> {
    /// In-band response payload, in whatever shape the caller selected.
    pub payload: P,
    /// Returned domain object IDs (empty for non-domain responses).
    pub objects: &'a [u32],
    /// Copy handles received.
    pub copy_handles: &'a [nx_svc::raw::Handle],
    /// Move handles received.
    pub move_handles: &'a [nx_svc::raw::Handle],
}

/// Error returned by [`parse_response`] and [`parse_response_domain`].
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// Response contains invalid CMIF magic header.
    #[error("invalid CMIF magic header")]
    InvalidMagic,
    /// Service returned a non-zero result code.
    #[error("service error: {0:#x}")]
    ServiceError(u32),
    /// Underlying HIPC layer rejected the response.
    #[error("HIPC parse: {0}")]
    Hipc(#[from] hipc::ResponseParseError),
    /// Response too small to contain a CMIF `OutHeader`.
    #[error("CMIF response too small for OutHeader")]
    TruncatedOutHeader,
    /// Response too small to contain a CMIF `DomainOutHeader`.
    #[error("CMIF response too small for DomainOutHeader")]
    TruncatedDomainHeader,
    /// Response too small to contain the caller-requested payload.
    #[error("CMIF response too small for payload")]
    TruncatedPayload,
    /// Response too small to contain the domain object IDs.
    #[error("CMIF response too small for domain objects")]
    TruncatedDomainObjects,
}

impl ToResultCode for ParseError {
    fn to_rc(self) -> ResultCode {
        match self {
            // The only variant carrying a code the server chose; every other
            // one is a shape this crate rejected after a successful reply.
            ParseError::ServiceError(code) => code,
            ParseError::Hipc(err) => err.to_rc(),
            // The one local failure libnx also detects, and it reports this.
            ParseError::InvalidMagic => libnx_error(LibnxError::InvalidCmifOutHeader),
            ParseError::TruncatedOutHeader
            | ParseError::TruncatedDomainHeader
            | ParseError::TruncatedPayload
            | ParseError::TruncatedDomainObjects => GENERIC_ERROR,
        }
    }
}

/// CMIF reply value.
///
/// Alias for a [`HipcReply`] carrying a [`CmifReplyBody`] payload. Serialize it
/// into the thread's IPC buffer with [`HipcReply::write_to`]; the server then
/// hands that buffer to the reply-and-receive syscall.
pub type CmifReply<P> = HipcReply<CmifReplyBody<P>>;

/// Fluent builder for a [`CmifReply`].
///
/// The result code is taken at construction rather than defaulted, because a
/// reply that forgot to report a failure is indistinguishable on the wire from
/// one that succeeded.
///
/// The payload is a [`HipcPayload`] rather than a byte slice so a reply can be
/// built from a value the caller owns. A handler computing a reply has nowhere
/// to put bytes it would then lend out, and `()` and `&[u8]` both implement the
/// trait, so the byte-slice case is unaffected.
pub struct CmifReplyBuilder<P: HipcPayload = ()> {
    hipc: HipcReplyBuilder,
    result: ResultCode,
    token: u32,
    payload: P,
}

impl CmifReplyBuilder {
    /// Starts a builder for a reply reporting `result`, with no payload.
    ///
    /// Attach one via [`with_payload`](Self::with_payload).
    #[inline]
    pub fn new(result: ResultCode) -> Self {
        Self {
            hipc: HipcReplyBuilder::new(),
            result,
            token: 0,
            payload: (),
        }
    }
}

impl<P: HipcPayload> CmifReplyBuilder<P> {
    /// Echoes the request's context token back to the client.
    ///
    /// The header's version field follows from it - `1` for a versioned
    /// request, `0` otherwise - so echoing the token a request carried is
    /// enough to answer it in the protocol version it used.
    #[inline]
    pub fn with_token(mut self, token: u32) -> Self {
        self.token = token;
        self
    }

    /// Attaches the reply payload, type-changing the builder to carry `Q`.
    ///
    /// All previously-accumulated envelope state is preserved.
    #[inline]
    pub fn with_payload<Q: HipcPayload>(self, payload: Q) -> CmifReplyBuilder<Q> {
        CmifReplyBuilder {
            hipc: self.hipc,
            result: self.result,
            token: self.token,
            payload,
        }
    }

    /// Sets the reply payload to bytes copied into the CMIF data area.
    #[inline]
    pub fn with_data(self, data: &[u8]) -> CmifReplyBuilder<&[u8]> {
        self.with_payload(data)
    }

    /// Sets the reply payload from a typed value via its zero-copy byte view.
    #[inline]
    pub fn with_data_value<T>(self, value: &T) -> CmifReplyBuilder<&[u8]>
    where
        T: zerocopy::IntoBytes + zerocopy::Immutable,
    {
        self.with_data(value.as_bytes())
    }

    /// Adds a send-static (Type X) descriptor carrying returned pointer data.
    ///
    /// # Panics
    ///
    /// Panics in debug builds once more than [`HIPC_MAX_DESCRIPTORS`] entries
    /// are added; the wire-format cap is hardware-fixed.
    ///
    /// [`HIPC_MAX_DESCRIPTORS`]: crate::hipc::HIPC_MAX_DESCRIPTORS
    #[inline]
    pub fn add_send_static(mut self, desc: StaticDescriptor) -> Self {
        self.hipc = self.hipc.with_send_static(desc);
        self
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
    pub fn build(self) -> CmifReply<P> {
        let body = CmifReplyBody {
            result: self.result,
            token: self.token,
            payload: self.payload,
        };
        self.hipc.with_payload(body).build()
    }
}

/// In-band body for a CMIF reply.
///
/// Encodes the alignment pad, the `OutHeader` carrying the `SFCO` magic and the
/// result code, and the reply payload after it. It has the same shape as the
/// control request body on the request side: no domain framing, no object-id
/// tail.
#[derive(Debug, Clone)]
pub struct CmifReplyBody<P: HipcPayload> {
    result: ResultCode,
    token: u32,
    payload: P,
}

impl<P: HipcPayload> CmifReplyBody<P> {
    /// Protocol version implied by the echoed token, matching how the request
    /// side derives it.
    fn cmif_version(&self) -> u32 {
        if self.token != 0 { 1 } else { 0 }
    }
}

impl<P: HipcPayload> HipcPayload for CmifReplyBody<P> {
    /// Alignment slack for the leading pad ([`CMIF_HEADER_ALIGN`]), plus the
    /// `OutHeader` and the reply payload.
    fn encoded_len(&self) -> usize {
        CMIF_HEADER_ALIGN + size_of::<OutHeader>() + self.payload.encoded_len()
    }

    /// Writes the CMIF reply body: skip the [`CMIF_HEADER_ALIGN`] alignment
    /// pad, then the `OutHeader` followed by the reply payload.
    fn write_to(&self, dst: &mut [u8]) {
        let region = &mut dst[..self.encoded_len()];

        // Skip alignment padding before the CMIF in-band header.
        let pad = region.as_ptr().align_offset(CMIF_HEADER_ALIGN);
        let (_padding, buf) = region.split_at_mut(pad);

        let header = OutHeader {
            magic: OUT_HEADER_MAGIC,
            version: self.cmif_version(),
            result: self.result,
            token: self.token,
        };
        let buf = write_section(buf, &header);

        self.payload.write_to(buf);
    }
}
