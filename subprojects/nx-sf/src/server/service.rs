//! The contract an interface implements to answer requests.

use super::{
    request::Request,
    response::Response,
};
use crate::hipc::HipcPayload;

/// An interface that answers requests.
///
/// The counterpart of `hyper`'s and `tower`'s `Service`: one method taking a
/// request and returning a response, with everything about ports, sessions and
/// the wire left to the runtime that calls it. An implementor writes what its
/// commands do and nothing else.
///
/// Not to be confused with [`crate::service`], which is the client-side handle
/// to a service someone *else* hosts. This trait is the hosting side.
///
/// # No failure in the signature
///
/// [`call`](Self::call) returns a [`Response`] rather than a `Result`, because
/// a failure is something this protocol *transmits* rather than something that
/// stops the exchange: the reply carries a result code either way, and a
/// command that failed is answered exactly as fully as one that succeeded. An
/// implementor reports failure by building a response around the code, which is
/// also what keeps a client from being left waiting on a session that answered
/// nothing.
///
/// # One body type per service
///
/// [`Body`](Self::Body) is fixed across the whole interface rather than per
/// command, and it is owned, so a reply cannot borrow from the request that
/// produced it. An interface whose commands return different shapes gives
/// `Body` an enum over them, or falls back to a byte buffer it fills.
pub trait Service {
    /// In-band body of the replies this service produces.
    type Body: HipcPayload;

    /// Answers one request.
    ///
    /// Takes `&mut self` because an interface is stateful: a session's whole
    /// point is that consecutive commands see each other's effects. The
    /// runtime calls this from a single thread, so nothing here needs to be
    /// synchronized.
    fn call(&mut self, request: Request<'_>) -> Response<Self::Body>;
}
