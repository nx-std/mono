//! CMIF (Command Message Interface Format) protocol implementation.
//!
//! CMIF is the command serialization layer built on top of HIPC. It provides
//! structured message formatting with magic headers for validation, command
//! IDs for method dispatch, and domain support for object multiplexing.
//!
//! Each submodule owns one message kind in both directions: `request` builds a
//! request as a client and parses one as a server, `response` parses a reply as
//! a client and builds one as a server. `wire` holds the layouts they share.
//!
//! # Server-side coverage
//!
//! The server path covers non-domain sessions: [`parse_request`] classifies a
//! message as a command, a control request, or a session close, and
//! [`CmifReplyBuilder`] answers it. Hosting a **domain** - converting a session,
//! keeping an object table, and emitting out-object ids - is not implemented.
//! A server built on this reports a failure result for the convert-to-domain
//! control request and keeps answering commands on the one interface the
//! session already names.

mod object_id;
mod request;
mod response;
mod wire;

pub use self::{
    object_id::ObjectId,
    request::{
        CmifCloseRequest,
        CmifControlRequestBuilder,
        CmifRequest,
        CmifRequestBuilder,
        Command,
        Request,
        RequestLayoutError,
        RequestParseError,
        SendError,
        parse_request,
    },
    response::{
        CmifReply,
        CmifReplyBody,
        CmifReplyBuilder,
        ParseError,
        Response,
        parse_response,
        parse_response_bytes,
        parse_response_domain,
        parse_response_domain_bytes,
    },
    wire::{
        CommandType,
        DomainInHeader,
        DomainOutHeader,
        DomainRequestType,
        InHeader,
        OutHeader,
    },
};
