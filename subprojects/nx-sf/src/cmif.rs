//! CMIF (Command Message Interface Format) protocol implementation.
//!
//! CMIF is the command serialization layer built on top of HIPC. It provides
//! structured message formatting with magic headers for validation, command
//! IDs for method dispatch, and domain support for object multiplexing.
//!
//! See the [`request`], [`response`], and [`wire`] submodules for the split
//! between builders, response parsing, and wire-format types.

mod object_id;
mod request;
mod response;
mod wire;

pub use self::{
    object_id::ObjectId,
    request::{
        CmifCloseRequest, CmifControlRequestBuilder, CmifRequest, CmifRequestBuilder,
        RequestLayoutError, SendError,
    },
    response::{
        ParseError, Response, parse_response, parse_response_bytes, parse_response_domain,
        parse_response_domain_bytes,
    },
    wire::{CommandType, DomainInHeader, DomainOutHeader, DomainRequestType, InHeader, OutHeader},
};
