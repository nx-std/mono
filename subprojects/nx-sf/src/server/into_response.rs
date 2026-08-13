//! Turning whatever a handler returns into a reply.

use super::{
    body::Body,
    response::Response,
};
use crate::{
    error::{
        ResultCode,
        ToResultCode,
    },
    hipc::HipcPayload,
};

/// Success code a reply reports when nothing said otherwise.
///
/// Zero is success across every Horizon result code, so a handler that returns
/// only a value is reporting one.
const SUCCESS: ResultCode = 0;

/// A value a handler may return in place of a [`Response`].
///
/// The counterpart of axum's trait of the same name, and it earns its keep the
/// same way: a handler says what it produced, not how a reply is shaped, and
/// the impls here cover the shapes worth writing.
///
/// Every conversion is infallible. A failure is not something this trait
/// reports - it is a result code, which is a perfectly ordinary response, so the
/// `Result` impl below turns one into a reply rather than into an error.
pub trait IntoResponse {
    /// Builds the reply this value stands for.
    fn into_response(self) -> Response<Body>;
}

impl IntoResponse for Response<Body> {
    #[inline]
    fn into_response(self) -> Self {
        self
    }
}

impl IntoResponse for () {
    /// A handler that returns nothing succeeded and has no body.
    #[inline]
    fn into_response(self) -> Response<Body> {
        Response::new(SUCCESS).with_body(Body::empty())
    }
}

impl IntoResponse for ResultCode {
    /// A bare code, success or failure, with no body.
    #[inline]
    fn into_response(self) -> Response<Body> {
        Response::new(self).with_body(Body::empty())
    }
}

impl<T, E> IntoResponse for Result<T, E>
where
    T: IntoResponse,
    E: ToResultCode,
{
    /// Reports the error as the reply's result code, since a failed command is
    /// answered as fully as one that succeeded.
    #[inline]
    fn into_response(self) -> Response<Body> {
        match self {
            Ok(value) => value.into_response(),
            Err(err) => err.to_rc().into_response(),
        }
    }
}

/// Wraps a payload as a successful reply's body.
///
/// The blanket `impl<P: HipcPayload> IntoResponse for P` this would rather be
/// cannot exist: it would overlap every impl above, since a payload type is not
/// distinguishable from a `Result` at the trait level. Naming the wrapper is
/// what keeps the other conversions reachable.
#[derive(Debug, Clone)]
pub struct Payload<P>(pub P);

impl<P: HipcPayload> IntoResponse for Payload<P> {
    #[inline]
    fn into_response(self) -> Response<Body> {
        Response::new(SUCCESS).with_body(Body::new(self.0))
    }
}
