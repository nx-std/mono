//! CMIF response parsing.

use nx_svc::error::ResultCode;
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
    hipc,
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
