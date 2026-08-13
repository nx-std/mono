//! Service Framework (SF) for Nintendo Switch
//!
//! This crate implements the **Service Framework** - the IPC serialization layer
//! used by Horizon OS services. The name "SF" comes from the CMIF protocol's
//! magic headers: `"SFCI"` (Service Framework Command Input) and `"SFCO"`
//! (Service Framework Command Output).
//!
//! # Architecture
//!
//! The IPC stack on Horizon OS is layered:
//!
//! ```text
//! ┌─────────────────────────────┐
//! │  Service APIs (fs, sm, etc) │  Application layer
//! ├─────────────────────────────┤
//! │  CMIF / TIPC                │  Command serialization (SF layer)
//! ├─────────────────────────────┤
//! │  HIPC                       │  Message framing & descriptors
//! ├─────────────────────────────┤
//! │  Kernel SVCs                │  Transport (SendSyncRequest, etc)
//! └─────────────────────────────┘
//! ```
//!
//! This crate provides the middle layers (HIPC, and CMIF/TIPC),
//! enabling Rust code to communicate with system services.
//!
//! # Protocols
//!
//! - **HIPC**: Low-level message format handling buffer descriptors, handles,
//!   and raw data layout. See the [`hipc`] module for details.
//! - **CMIF**: Command interface with domain support (object multiplexing).
//!   Uses `"SFCI"`/`"SFCO"` magic headers. See the [`cmif`] module for details.
//! - **TIPC**: Simplified protocol introduced in HOS 12.0.0. No domains,
//!   command ID stored in HIPC message type. See the [`tipc`] module for details.
//!
//! # Hosting an interface: the layering this crate is growing into
//!
//! Serving IPC and serving HTTP are the same problem wearing different wire
//! formats: a framed message arrives, something decides which method it names,
//! a handler runs, and a status-carrying reply goes back. Rust already has a
//! well-worn three-crate answer to that problem, and this crate is being
//! shaped to match it layer for layer:
//!
//! | Rust web stack | here |
//! |---|---|
//! | `http` - one `Request`/`Response` pair every protocol version decodes into | `server::Request` / `server::Response` |
//! | `http`'s `Method`, `StatusCode`, `Version` | `server::CommandId`, [`error::ResultCode`], `server::Protocol` |
//! | `hyper`'s per-version codecs (`h1`, `h2`) | [`cmif`] and [`tipc`], both framing over [`hipc`] |
//! | `hyper`'s connection runtime and `Service` trait | not yet written |
//! | `axum`'s `Router`, handlers and extractors | not yet written |
//!
//! The payoff is the same one the web stack gets from it. A handler is written
//! against one request type and one reply type, and stays correct whether the
//! client on the other end speaks CMIF or TIPC - exactly as an `axum` handler
//! is indifferent to HTTP/1.1 versus HTTP/2. The protocol modules keep their
//! own types, because their wire formats genuinely differ; what the `server`
//! module adds is the single shape above them a handler can be written
//! against.
//!
//! The `server` module holds the message half of that picture today, behind
//! the `server` feature. The runtime and the router are later work.

#![no_std]

extern crate alloc;
extern crate nx_panic_handler;
// Provides #[panic_handler]

mod array_vec;
pub mod cmif;
mod cursor;
pub mod error;
pub mod hipc;
pub mod ipc;
#[cfg(feature = "server")]
pub mod server;
pub mod service;
mod service_name;
pub mod tipc;

pub use self::{
    cursor::{
        Cursor,
        ResponsePayload,
    },
    service::{
        Domain,
        DomainObject,
        OverrideService,
        Session,
    },
    service_name::ServiceName,
};

#[cfg(feature = "ffi")]
pub mod ffi;
