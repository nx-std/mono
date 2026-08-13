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
//! # What a handler may rely on: the portable surface is TIPC's
//!
//! The two protocols are not peers with a shared middle. CMIF is a strict
//! superset: everything TIPC does - invoke a command by id, carry argument
//! bytes and mapped buffers, pass copy and move handles, return a result code,
//! hand back a sub-object as a move handle - CMIF also does. What TIPC drops is
//! the machinery around that, not any of it: no magic to validate, no in-band
//! header, no domains, no control requests, no pointer descriptors, no context
//! token. It exists to be cheaper, not to be different.
//!
//! So the set of things a handler can do and still serve either client is
//! exactly TIPC's, and that makes the contract statable rather than a matter of
//! comparing two protocol modules: **write to TIPC's capabilities and one
//! handler serves both**. Everything CMIF adds is an opt-out, not a feature to
//! reach for and hope.
//!
//! Two consequences worth knowing, because they are the ones that surprise:
//!
//! - **Returning an object is portable.** A domain hands back an object id and
//!   is CMIF-only, but a *non-domain* CMIF session returns a sub-object as a
//!   move handle, exactly as TIPC does. Since [`Server`] refuses domain
//!   conversion, every session it hosts is non-domain, so this needs no branch
//!   at all.
//! - **Returning pointer data is not.** Send statics are the one CMIF-only
//!   facility a [`Response`] can currently express, and encoding such a reply
//!   for a TIPC session fails with [`IncompatibleProtocolError`].
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
//!
//! # Running one
//!
//! [`Service`] is the trait an interface implements, and [`Server`] is the loop
//! that feeds it: it owns the port, accepts the sessions arriving on it, and
//! drives the kernel call that both sends the previous reply and waits for the
//! next request. Together they are the `hyper` half of the picture.
//!
//! [`Router`] is the `axum` half above them: one handler per [`CommandId`],
//! each an ordinary function whose parameters are extracted from the request.
//! A `Router` is itself a `Service`, so hosting one is hosting an interface.

mod body;
mod command;
mod extract;
mod handler;
mod into_response;
mod protocol;
mod request;
mod response;
mod router;
mod serve;
mod service;

pub use self::{
    body::Body,
    command::CommandId,
    extract::{
        Args,
        ArgsRejection,
        FromRequest,
        FromRequestParts,
        State,
    },
    handler::Handler,
    into_response::{
        IntoResponse,
        Payload,
    },
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
        IncompatibleProtocolError,
        Reply,
        Response,
    },
    router::Router,
    serve::{
        ServeError,
        Server,
    },
    service::Service,
};
