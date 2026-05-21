//! CMIF response parsing.

use nx_sys_thread_tls::IPC_BUFFER_SIZE;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use super::wire::{DomainOutHeader, OUT_HEADER_MAGIC, OutHeader};
use crate::hipc;

/// Parsed CMIF response with typed payload.
#[derive(Debug)]
pub struct Response<'a, T> {
    /// Typed response payload.
    pub payload: &'a T,
    /// Returned domain object IDs.
    pub objects: &'a [u32],
    /// Copy handles received.
    pub copy_handles: &'a [nx_svc::raw::Handle],
    /// Move handles received.
    pub move_handles: &'a [nx_svc::raw::Handle],
}

/// Parsed CMIF response with byte payload.
#[derive(Debug)]
pub struct ResponseBytes<'a> {
    /// Raw response payload bytes.
    pub data: &'a [u8],
    /// Returned domain object IDs.
    pub objects: &'a [u32],
    /// Copy handles received.
    pub copy_handles: &'a [nx_svc::raw::Handle],
    /// Move handles received.
    pub move_handles: &'a [nx_svc::raw::Handle],
}

/// Error returned by [`parse_response`].
#[derive(Debug, thiserror::Error)]
pub enum ParseRespError {
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
    /// Response too small to contain the typed payload `T`.
    #[error("CMIF response too small for payload")]
    TruncatedPayload,
    /// Response too small to contain the domain object IDs.
    #[error("CMIF response too small for domain objects")]
    TruncatedDomainObjects,
}

/// Error returned by [`parse_response_bytes`].
#[derive(Debug, thiserror::Error)]
pub enum ParseRespBytesError {
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
    /// Response too small to contain the caller-requested payload size.
    #[error("CMIF response too small for payload")]
    TruncatedPayload,
    /// Response too small to contain the domain object IDs.
    #[error("CMIF response too small for domain objects")]
    TruncatedDomainObjects,
}

/// Parses a CMIF non-domain response message into a typed payload.
pub fn parse_response<'a, T>(
    buf: &'a [u8; IPC_BUFFER_SIZE],
) -> Result<Response<'a, T>, ParseRespError>
where
    T: FromBytes + Immutable + KnownLayout,
{
    let hipc_resp = hipc::parse_response(buf)?;

    let data_bytes: &'a [u8] = hipc_resp.data_words.as_bytes();
    let pad = data_bytes.as_ptr().align_offset(16);
    let (_padding, aligned) = data_bytes.split_at(pad);

    let (out_hdr_slot, rest) =
        OutHeader::ref_from_prefix(aligned).map_err(|_| ParseRespError::TruncatedOutHeader)?;
    let (payload, _) = T::ref_from_prefix(rest).map_err(|_| ParseRespError::TruncatedPayload)?;

    validate_out_header(out_hdr_slot)?;

    Ok(Response {
        payload,
        objects: &[],
        copy_handles: hipc_resp.copy_handles,
        move_handles: hipc_resp.move_handles,
    })
}

/// Parses a CMIF domain response message into a typed payload.
pub fn parse_response_domain<'a, T>(
    buf: &'a [u8; IPC_BUFFER_SIZE],
) -> Result<Response<'a, T>, ParseRespError>
where
    T: FromBytes + Immutable + KnownLayout,
{
    let hipc_resp = hipc::parse_response(buf)?;
    let data_bytes: &'a [u8] = hipc_resp.data_words.as_bytes();
    let pad = data_bytes.as_ptr().align_offset(16);
    let (_padding, aligned) = data_bytes.split_at(pad);

    let (domain_hdr, rest) = DomainOutHeader::ref_from_prefix(aligned)
        .map_err(|_| ParseRespError::TruncatedDomainHeader)?;
    let (out_hdr_slot, rest) =
        OutHeader::ref_from_prefix(rest).map_err(|_| ParseRespError::TruncatedOutHeader)?;
    let (payload, rest) = T::ref_from_prefix(rest).map_err(|_| ParseRespError::TruncatedPayload)?;
    let count = domain_hdr.num_out_objects as usize;
    let (objects, _) = <[u32]>::ref_from_prefix_with_elems(rest, count)
        .map_err(|_| ParseRespError::TruncatedDomainObjects)?;

    validate_out_header(out_hdr_slot)?;

    Ok(Response {
        payload,
        objects,
        copy_handles: hipc_resp.copy_handles,
        move_handles: hipc_resp.move_handles,
    })
}

/// Parses a CMIF non-domain response message with a runtime-sized byte payload.
pub fn parse_response_bytes<'a>(
    buf: &'a [u8; IPC_BUFFER_SIZE],
    size: usize,
) -> Result<ResponseBytes<'a>, ParseRespBytesError> {
    let hipc_resp = hipc::parse_response(buf)?;
    let data_bytes: &'a [u8] = hipc_resp.data_words.as_bytes();
    let pad = data_bytes.as_ptr().align_offset(16);
    let (_padding, aligned) = data_bytes.split_at(pad);

    let (out_hdr_slot, rest) =
        OutHeader::ref_from_prefix(aligned).map_err(|_| ParseRespBytesError::TruncatedOutHeader)?;
    let (data, _) = rest
        .split_at_checked(size)
        .ok_or(ParseRespBytesError::TruncatedPayload)?;

    validate_out_header_bytes(out_hdr_slot)?;

    Ok(ResponseBytes {
        data,
        objects: &[],
        copy_handles: hipc_resp.copy_handles,
        move_handles: hipc_resp.move_handles,
    })
}

/// Parses a CMIF domain response message with a runtime-sized byte payload.
pub fn parse_response_bytes_domain<'a>(
    buf: &'a [u8; IPC_BUFFER_SIZE],
    size: usize,
) -> Result<ResponseBytes<'a>, ParseRespBytesError> {
    let hipc_resp = hipc::parse_response(buf)?;
    let data_bytes: &'a [u8] = hipc_resp.data_words.as_bytes();
    let pad = data_bytes.as_ptr().align_offset(16);
    let (_padding, aligned) = data_bytes.split_at(pad);

    let (domain_hdr, rest) = DomainOutHeader::ref_from_prefix(aligned)
        .map_err(|_| ParseRespBytesError::TruncatedDomainHeader)?;
    let (out_hdr_slot, rest) =
        OutHeader::ref_from_prefix(rest).map_err(|_| ParseRespBytesError::TruncatedOutHeader)?;
    let (data, rest) = rest
        .split_at_checked(size)
        .ok_or(ParseRespBytesError::TruncatedPayload)?;
    let count = domain_hdr.num_out_objects as usize;
    let (objects, _) = <[u32]>::ref_from_prefix_with_elems(rest, count)
        .map_err(|_| ParseRespBytesError::TruncatedDomainObjects)?;

    validate_out_header_bytes(out_hdr_slot)?;

    Ok(ResponseBytes {
        data,
        objects,
        copy_handles: hipc_resp.copy_handles,
        move_handles: hipc_resp.move_handles,
    })
}

#[inline]
fn validate_out_header(hdr: &OutHeader) -> Result<(), ParseRespError> {
    if hdr.magic != OUT_HEADER_MAGIC {
        return Err(ParseRespError::InvalidMagic);
    }
    if hdr.result != 0 {
        return Err(ParseRespError::ServiceError(hdr.result));
    }
    Ok(())
}

#[inline]
fn validate_out_header_bytes(hdr: &OutHeader) -> Result<(), ParseRespBytesError> {
    if hdr.magic != OUT_HEADER_MAGIC {
        return Err(ParseRespBytesError::InvalidMagic);
    }
    if hdr.result != 0 {
        return Err(ParseRespBytesError::ServiceError(hdr.result));
    }
    Ok(())
}
