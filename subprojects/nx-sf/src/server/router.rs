//! Dispatching a command to the handler registered for it.

use alloc::{
    boxed::Box,
    collections::BTreeMap,
};

use super::{
    body::Body,
    command::CommandId,
    handler::Handler,
    into_response::IntoResponse,
    request::Request,
    response::Response,
    service::Service,
};
use crate::error::{
    LibnxError,
    ResultCode,
    libnx_error,
};

/// Result code reported for a command no route claims.
///
/// The interface is reachable and the message decoded; what is missing is a
/// handler for the id the client named, which is what a caller addressing
/// something that is not registered gets told.
const UNKNOWN_COMMAND: ResultCode = libnx_error(LibnxError::NotFound);

/// A boxed handler, ready to be called with a request.
///
/// The `for<'a>` is what lets one stored value serve requests borrowing
/// different messages, and the box is what lets routes of different handler
/// types sit in one map. Both are paid once per registration; the virtual call
/// is the per-request cost, and it is the cost of routing at all.
type Route<S> = Box<dyn for<'a> Fn(Request<'a>, &S) -> Response<Body>>;

/// An interface assembled from one handler per command.
///
/// The counterpart of axum's `Router`, keyed by [`CommandId`] where axum keys
/// by path and method. It is itself a [`Service`], so a
/// [`Server`](super::Server) hosts one exactly as it would host a hand-written
/// interface.
///
/// # State
///
/// Unlike axum, the state is supplied at construction rather than attached
/// afterwards. axum defers it because a router is assembled from nested routers
/// that each need their own; nothing here nests, so taking it up front spares
/// the type parameter that would otherwise track whether it has been supplied.
pub struct Router<S> {
    routes: BTreeMap<CommandId, Route<S>>,
    state: S,
}

impl<S> Router<S> {
    /// Starts an interface with no commands, over `state`.
    #[inline]
    pub fn new(state: S) -> Self {
        Self {
            routes: BTreeMap::new(),
            state,
        }
    }

    /// Registers `handler` as the command `id` names.
    ///
    /// Registering an id twice keeps the later handler, which is what a reader
    /// of the builder chain would expect from the order it is written in.
    ///
    /// The id is a [`CommandId`] rather than anything an integer converts into,
    /// so a route table reads as a list of commands rather than a list of
    /// numbers that happen to be in the right order.
    pub fn command<T, H>(mut self, id: CommandId, handler: H) -> Self
    where
        H: Handler<T, S> + 'static,
        T: 'static,
        S: 'static,
    {
        let route: Route<S> = Box::new(move |request, state| {
            // Cloned per call because `Handler::call` consumes the handler: a
            // function item or a closure that captures nothing clones to
            // nothing, which is what the common case costs.
            handler.clone().call(request, state)
        });
        self.routes.insert(id, route);
        self
    }

    /// Returns the state the routes are called with.
    #[inline]
    pub fn state(&self) -> &S {
        &self.state
    }
}

impl<S> Service for Router<S> {
    type Body = Body;

    fn call(&mut self, request: Request<'_>) -> Response<Self::Body> {
        match self.routes.get(&request.command()) {
            Some(route) => route(request, &self.state),
            // Answered rather than dropped: the client is waiting on a reply
            // whatever the id turned out to be.
            None => UNKNOWN_COMMAND.into_response(),
        }
    }
}
