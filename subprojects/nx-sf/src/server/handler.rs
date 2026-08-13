//! Ordinary functions, callable as commands.

use super::{
    body::Body,
    extract::{
        FromRequest,
        FromRequestParts,
    },
    into_response::IntoResponse,
    request::Request,
    response::Response,
};

/// A function that can answer a command.
///
/// The counterpart of axum's trait of the same name, and it exists for the same
/// reason: a command handler should be a plain function whose parameters say
/// what it needs, not a trait impl that takes a request and digs. The impls
/// below make that work by extracting each parameter from the request before
/// the function is called.
///
/// `T` is the tuple of parameter types and never named at a call site; it is
/// what lets one trait cover functions of different arities. It also carries
/// the marker that says how the last parameter reaches
/// [`FromRequest`](super::FromRequest), which is why the tuple has one more
/// member than the function has parameters. `S` is the interface state those
/// parameters may be extracted from.
///
/// # The parameter split
///
/// Every parameter but the last is a [`FromRequestParts`], which reads the
/// request head. The last may additionally be a [`FromRequest`], which reads the
/// body. That is axum's rule, and the reason is the same here: the body is one
/// value and can only be handed to one extractor.
pub trait Handler<T, S>: Clone {
    /// Extracts this handler's parameters from `request` and calls it.
    ///
    /// Returns the rejection's reply instead if any parameter cannot be
    /// extracted, so a handler never sees a request its own signature does not
    /// describe.
    fn call(self, request: Request<'_>, state: &S) -> Response<Body>;
}

impl<F, R, S> Handler<(), S> for F
where
    F: FnOnce() -> R + Clone,
    R: IntoResponse,
{
    fn call(self, _: Request<'_>, _: &S) -> Response<Body> {
        self().into_response()
    }
}

/// Writes the [`Handler`] impl for a function of one or more parameters.
///
/// Every impl is the same shape: extract each leading parameter from the head,
/// extract the last from the whole request, and return the first rejection
/// instead of calling. Only the number of parameters differs, and a trait impl
/// cannot be generic over that, so the arities are generated rather than
/// written out. This is the one place in the crate where a macro stands in for
/// code a reader would otherwise scroll past sixteen near-identical copies of.
macro_rules! impl_handler {
    ( [$($head:ident),*], $last:ident ) => {
        #[expect(
            non_snake_case,
            reason = "the generated bindings reuse the type parameter names, which is what \
                      keeps each expansion readable against the impl header above it"
        )]
        impl<F, R, S, M, $($head,)* $last> Handler<(M, $($head,)* $last), S> for F
        where
            F: FnOnce($($head,)* $last) -> R + Clone,
            R: IntoResponse,
            $( $head: FromRequestParts<S>, )*
            $last: FromRequest<S, M>,
        {
            fn call(self, request: Request<'_>, state: &S) -> Response<Body> {
                $(
                    let $head = match $head::from_request_parts(request.parts(), state) {
                        Ok(value) => value,
                        Err(rejection) => return rejection.into_response(),
                    };
                )*

                let $last = match $last::from_request(request, state) {
                    Ok(value) => value,
                    Err(rejection) => return rejection.into_response(),
                };

                self($($head,)* $last).into_response()
            }
        }
    };
}

// The bracketed list is the head parameters and the name after it is the one
// that may read the body. The brackets are not decoration: without them the
// matcher cannot tell where the repetition ends and the final name begins, and
// every invocation is a parse ambiguity.
impl_handler!([], T1);
impl_handler!([T1], T2);
impl_handler!([T1, T2], T3);
impl_handler!([T1, T2, T3], T4);
impl_handler!([T1, T2, T3, T4], T5);
impl_handler!([T1, T2, T3, T4, T5], T6);
impl_handler!([T1, T2, T3, T4, T5, T6], T7);
impl_handler!([T1, T2, T3, T4, T5, T6, T7], T8);
