//! Pulling a handler's arguments out of a request.

use super::{
    into_response::IntoResponse,
    request::{
        Parts,
        Request,
    },
};
use crate::error::{
    GENERIC_ERROR,
    ResultCode,
    ToResultCode,
};

/// An argument a handler can take alongside the ones before it.
///
/// The counterpart of axum's `FromRequestParts`: it reads the request head,
/// which every extractor may do, so a handler may take any number of these.
/// Reading the body is [`FromRequest`]'s job, and a handler takes at most one
/// of those.
///
/// # Extractors are owned
///
/// A request borrows the IPC buffer, so an extractor could in principle hand a
/// handler a borrow into the message and copy nothing. This trait deliberately
/// does not: it takes the head by reference and returns an owned value, so the
/// trait carries no lifetime, and neither does [`Handler`](super::Handler) or
/// anything the router stores. Threading one through would put it on the
/// handler trait, on its generated arity impls, and on every boxed route, to
/// save copies of things the wire format already bounds: a command's arguments
/// are a few words, and the descriptor and handle sections cap at fifteen
/// entries each.
///
/// What it costs is that [`Request`] itself cannot be an extractor, where in
/// axum it is: a handler cannot ask for the whole message, only for pieces
/// something implements this trait for. A command needing something no
/// extractor exposes yet wants a new extractor; one needing the message
/// entire is asking to be a [`Service`](super::Service), which is the layer
/// below and still open to it.
pub trait FromRequestParts<S>: Sized {
    /// What this extractor answers with when the request does not carry what it
    /// needs.
    type Rejection: IntoResponse;

    /// Reads the argument out of the request head.
    ///
    /// # Errors
    ///
    /// Returns [`Rejection`](Self::Rejection), which the router turns into the
    /// reply in place of calling the handler.
    fn from_request_parts(parts: &Parts<'_>, state: &S) -> Result<Self, Self::Rejection>;
}

/// Which route an extractor took to become a [`FromRequest`].
///
/// The marker exists to keep the blanket impl below from overlapping the
/// specific ones: without it, "every head extractor is also a whole-request
/// extractor" and "`Args` is a whole-request extractor" are two impls of one
/// trait for types that might coincide, which the coherence rules reject. Each
/// route gets its own marker, so the two impls differ in a type parameter and
/// no longer collide. `M` is inferred at every call site and never written.
///
/// This is the shape axum uses, for the same collision.
mod via {
    /// Reached through [`FromRequestParts`](super::FromRequestParts).
    #[derive(Debug, Clone, Copy)]
    pub enum Parts {}

    /// Implemented directly, reading the body.
    #[derive(Debug, Clone, Copy)]
    pub enum Request {}
}

/// The last argument a handler takes, which may read the body.
///
/// The counterpart of axum's `FromRequest`, and it splits from
/// [`FromRequestParts`] for the same reason: only one argument may consume the
/// body, so only the last position can be one of these.
pub trait FromRequest<S, M = via::Request>: Sized {
    /// What this extractor answers with when the request does not carry what it
    /// needs.
    type Rejection: IntoResponse;

    /// Reads the argument out of the whole request.
    ///
    /// # Errors
    ///
    /// Returns [`Rejection`](Self::Rejection), which the router turns into the
    /// reply in place of calling the handler.
    fn from_request(request: Request<'_>, state: &S) -> Result<Self, Self::Rejection>;
}

/// Every head extractor is also a whole-request extractor, by ignoring the
/// body. This is what lets a handler's last argument be either kind.
impl<S, T: FromRequestParts<S>> FromRequest<S, via::Parts> for T {
    type Rejection = <Self as FromRequestParts<S>>::Rejection;

    #[inline]
    fn from_request(request: Request<'_>, state: &S) -> Result<Self, Self::Rejection> {
        Self::from_request_parts(request.parts(), state)
    }
}

/// The interface's own state, handed to a handler that needs it.
///
/// The counterpart of axum's extractor of the same name, and it copies the same
/// way: `S` is cloned out of what the router holds, so an interface whose state
/// is expensive to copy stores a handle to it rather than the thing itself.
///
/// # It is a snapshot, not a handle to write through
///
/// A handler receives a clone, so assigning to it changes nothing a later
/// command will see. This extractor is for what an interface *reads*: the
/// configuration it was built with, a handle it holds, a limit it enforces.
///
/// An interface whose commands exist to change something shared is not asking
/// for this extractor. It is asking to be a [`Service`](super::Service), whose
/// `call` takes `&mut self` and where consecutive commands see each other's
/// effects with nothing wrapped around them. That layer is below this one and
/// stays available; a [`Router`](super::Router) is the convenience for the
/// common case, not the only way to host an interface.
///
/// The alternative - routes taking `&mut S` - was not taken, because an
/// extractor reads the state by shared reference and only one of a handler's
/// parameters could hold the unique one, which is the same restriction the
/// body already carries and a second place for it to surprise someone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State<S>(pub S);

impl<S: Clone> FromRequestParts<S> for State<S> {
    /// Reading state cannot fail: the router holds it, so it is always there.
    type Rejection = core::convert::Infallible;

    #[inline]
    fn from_request_parts(_: &Parts<'_>, state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(state.clone()))
    }
}

impl IntoResponse for core::convert::Infallible {
    fn into_response(self) -> super::Response<super::Body> {
        match self {}
    }
}

/// The command's arguments, read from the data words as a `T`.
///
/// The counterpart of axum's `Json`: the extractor that reads the body as a
/// declared shape. `T` is a zerocopy struct, so this is one bounds check and a
/// copy rather than a parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Args<T>(pub T);

impl<S, T> FromRequest<S> for Args<T>
where
    T: zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    type Rejection = ArgsRejection;

    fn from_request(request: Request<'_>, _: &S) -> Result<Self, Self::Rejection> {
        // The body still carries whatever padding the protocol left after its
        // header, so the read takes a prefix rather than the whole of it.
        let (value, _) = T::read_from_prefix(request.body()).map_err(|_| ArgsRejection)?;
        Ok(Self(value))
    }
}

/// Rejection returned by [`Args`].
///
/// Occurs when the data words hold fewer bytes than the declared argument
/// struct, which is a client that sent a message the command's signature does
/// not describe. Nothing was read.
#[derive(Debug, thiserror::Error)]
#[error("the request body is too short for the command's arguments")]
pub struct ArgsRejection;

impl ToResultCode for ArgsRejection {
    fn to_rc(self) -> ResultCode {
        // A request rejected before the handler ran, so no service assigned it
        // a code to forward.
        GENERIC_ERROR
    }
}

impl IntoResponse for ArgsRejection {
    #[inline]
    fn into_response(self) -> super::Response<super::Body> {
        self.to_rc().into_response()
    }
}
