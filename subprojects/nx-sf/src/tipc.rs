//! TIPC (Trivial IPC) protocol implementation.
//!
//! TIPC is a simplified IPC protocol introduced in Horizon OS 12.0.0. Unlike
//! CMIF, it has no domain support and stores the command ID directly in the
//! HIPC message type field.
//!
//! Each submodule owns one message kind in both directions: `request` builds a
//! request as a client and parses one as a server, `response` parses a reply as
//! a client and builds one as a server. `wire` holds the layouts they share.
//!
//! # Protocol Stack
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │  Service APIs (fs, sm, hid, etc.)   │  Application layer
//! ├─────────────────────────────────────┤
//! │  TIPC  ← this module                │  Command serialization
//! ├─────────────────────────────────────┤
//! │  HIPC                               │  Message framing & descriptors
//! ├─────────────────────────────────────┤
//! │  Kernel SVCs (SendSyncRequest, etc) │  Transport
//! └─────────────────────────────────────┘
//! ```
//!
//! # Key Differences from CMIF
//!
//! | Aspect              | CMIF                     | TIPC                      |
//! |---------------------|--------------------------|---------------------------|
//! | Command ID          | In CMIF header           | HIPC message type (ID+16) |
//! | Domain support      | Yes                      | No                        |
//! | Magic headers       | SFCI/SFCO                | None                      |
//! | Close command       | Type=2                   | Type=15                   |
//! | Pointer descriptors | Type X/C (statics)       | None                      |
//! | Result code         | In OutHeader.result      | First u32 of data words   |
//! | Object passing      | Domain object IDs        | Move handles              |
//!
//! # Message Format
//!
//! **Request:**
//! ```text
//! [HIPC Header (type = command_id + 16)]
//! [HIPC Descriptors (buffers, handles)]
//! [Data Words (raw payload)]
//! ```
//!
//! **Response:**
//! ```text
//! [HIPC Header]
//! [HIPC Descriptors (handles)]
//! [Result Code (u32)]
//! [Response Payload]
//! ```
//!
//! # Builder model
//!
//! [`TipcRequestBuilder`] is the high-level entry point for TIPC requests. It
//! wraps a [`hipc::HipcRequestBuilder`] and exposes only the descriptor kinds
//! TIPC supports (mapped buffers + copy handles). Build a [`TipcRequest`] with
//! [`TipcRequestBuilder::build`] and serialize it with [`TipcRequest::write_to`].
//!
//! # Server-side coverage
//!
//! [`parse_request`] classifies an inbound message as a command or a session
//! close, and [`TipcReplyBuilder`] answers it. There is nothing further to
//! cover: TIPC has no domains and no control requests, so the server path is
//! complete where CMIF's is not.
//!
//! # References
//!
//! - [Switchbrew IPC Marshalling](https://switchbrew.org/wiki/IPC_Marshalling)
//! - libnx `sf/tipc.h` (fincs, SciresM)

mod request;
mod response;
mod wire;

pub use self::{
    request::{
        Command,
        Request,
        RequestLayoutError,
        RequestParseError,
        SendError,
        TipcCloseRequest,
        TipcRequest,
        TipcRequestBuilder,
        parse_request,
    },
    response::{
        ParseResponseError,
        Response,
        TipcReply,
        TipcReplyBody,
        TipcReplyBuilder,
        parse_response,
    },
    wire::CommandType,
};
