//! Protocol-neutral message types for hosting an interface.
//!
//! [`cmif`](crate::cmif) and [`tipc`](crate::tipc) each decode an inbound
//! message into types of their own, and the two disagree in every place their
//! wire formats do: a context token exists in one and not the other, a command
//! id is read from an in-band header in one and from the message type in the
//! other, control requests exist only in the first. Written against those types
//! directly, a handler is written twice.
//!
//! This module is the one shape both decode into, in the role `http::Request`
//! and `http::Response` play for `hyper`'s two protocol codecs. A handler takes
//! a [`Request`] and returns a [`Response`], and the protocol survives only as
//! the [`Protocol`] value in the request head - carried so the reply can be
//! encoded the way the client will read it, not so the handler can branch on
//! it.
//!
//! # Head and body
//!
//! A [`Request`] is [`Parts`] plus a body, the split `http` uses. What lands on
//! which side is decided by what a handler needs to reach without consuming the
//! message:
//!
//! - **Head** ([`Parts`]): the command id, the protocol, the sender's process
//!   id, and every descriptor and handle the request carries. All of it is
//!   `Copy`-cheap borrows of the message buffer.
//! - **Body**: the argument bytes, which is the data-words region past whatever
//!   in-band header the protocol wrote.
//!
//! Handles and buffer descriptors sit in the head rather than the body even
//! though they carry payload data, because a command routinely takes both a
//! handle argument and a data argument. Anything a handler may need alongside
//! the arguments has to be reachable without consuming the message.
//!
//! # Direction
//!
//! [`Request`] borrows the message buffer and is only ever parsed; [`Response`]
//! is only ever built. That asymmetry is deliberate and mirrors the one
//! `hyper` has for a server: the message that arrives is bytes somebody else
//! wrote, and the message that leaves is a value this process assembles.
//! Hosting one end of a session means each direction is only ever handled one
//! way, so a type that did both would carry a dead half.

mod command;
mod protocol;
mod request;
mod response;

pub use self::{
    command::CommandId,
    protocol::{
        CmifVersion,
        Protocol,
    },
    request::{
        Inbound,
        Parts,
        Request,
        RequestParseError,
        parse_request,
    },
    response::{
        PointerDataOverTipcError,
        Reply,
        Response,
    },
};
