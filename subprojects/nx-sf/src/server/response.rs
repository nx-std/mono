//! Outbound replies, in the shape a handler produces them.
//!
//! A handler builds a [`Response`] without knowing which protocol will carry
//! it. [`Response::into_reply`] performs the encoding, taking the protocol from
//! the request head that produced the response.

use nx_svc::raw::Handle as RawHandle;

use super::protocol::{
    CmifVersion,
    Protocol,
};
use crate::{
    array_vec::ArrayVec,
    cmif::{
        CmifReply,
        CmifReplyBuilder,
    },
    error::{
        GENERIC_ERROR,
        ResultCode,
        ToResultCode,
    },
    hipc::{
        HIPC_MAX_DESCRIPTORS,
        HipcPayload,
        StaticDescriptor,
    },
    tipc::{
        TipcReply,
        TipcReplyBuilder,
    },
};

/// A reply a handler has produced, before it is encoded for the wire.
///
/// The counterpart of `http::Response`: a result code in the role of the status
/// code, a body, and the handles and pointer data that ride alongside.
///
/// The body is owned rather than borrowed, and generic for the same reason
/// `http::Response` is: a handler computes its reply and has nowhere to put
/// bytes it would then lend out. `()` covers a reply that carries none, `&[u8]`
/// a reply forwarding bytes it already holds, and any [`HipcPayload`] a value
/// the handler built.
///
/// The result code is taken at construction rather than defaulted, because a
/// reply that forgot to report a failure is indistinguishable on the wire from
/// one that succeeded.
#[derive(Debug, Clone)]
pub struct Response<B: HipcPayload = ()> {
    result: ResultCode,
    body: B,
    copy_handles: ArrayVec<RawHandle, HIPC_MAX_DESCRIPTORS>,
    move_handles: ArrayVec<RawHandle, HIPC_MAX_DESCRIPTORS>,
    send_statics: ArrayVec<StaticDescriptor, HIPC_MAX_DESCRIPTORS>,
}

impl Response {
    /// Starts a reply reporting `result`, with no body and nothing attached.
    ///
    /// Attach a body via [`with_body`](Self::with_body).
    #[inline]
    pub fn new(result: ResultCode) -> Self {
        Self {
            result,
            body: (),
            copy_handles: ArrayVec::new(),
            move_handles: ArrayVec::new(),
            send_statics: ArrayVec::new(),
        }
    }
}

impl<B: HipcPayload> Response<B> {
    /// Attaches the reply body, type-changing the response to carry `C`.
    ///
    /// All previously-attached handles and pointer data are preserved.
    #[inline]
    pub fn with_body<C: HipcPayload>(self, body: C) -> Response<C> {
        Response {
            result: self.result,
            body,
            copy_handles: self.copy_handles,
            move_handles: self.move_handles,
            send_statics: self.send_statics,
        }
    }

    /// Attaches a copy handle, which the kernel duplicates into the client.
    ///
    /// # Panics
    ///
    /// Panics in debug builds once more than [`HIPC_MAX_DESCRIPTORS`] entries
    /// are attached; the wire-format cap is hardware-fixed.
    #[inline]
    pub fn with_copy_handle(mut self, handle: RawHandle) -> Self {
        self.copy_handles.push(handle);
        self
    }

    /// Attaches a move handle, transferring ownership to the client.
    ///
    /// # Panics
    ///
    /// Panics in debug builds once more than [`HIPC_MAX_DESCRIPTORS`] entries
    /// are attached; the wire-format cap is hardware-fixed.
    #[inline]
    pub fn with_move_handle(mut self, handle: RawHandle) -> Self {
        self.move_handles.push(handle);
        self
    }

    /// Attaches a send static (Type X) carrying returned pointer data.
    ///
    /// Only CMIF can carry these; see [`into_reply`](Self::into_reply).
    ///
    /// # Panics
    ///
    /// Panics in debug builds once more than [`HIPC_MAX_DESCRIPTORS`] entries
    /// are attached; the wire-format cap is hardware-fixed.
    #[inline]
    pub fn with_send_static(mut self, desc: StaticDescriptor) -> Self {
        self.send_statics.push(desc);
        self
    }

    /// Returns the result code this reply reports.
    #[inline]
    pub fn result(&self) -> ResultCode {
        self.result
    }

    /// Returns the reply body.
    #[inline]
    pub fn body(&self) -> &B {
        &self.body
    }

    /// Encodes the reply for `protocol`, which is the one the request arrived
    /// over.
    ///
    /// Pass the [`Protocol`] out of the request head rather than choosing one:
    /// a client reads the reply in the protocol it wrote the request in, and
    /// nothing else in the message says which that was.
    ///
    /// Consumes the response because the body is owned and the encoded reply
    /// takes it over rather than copying it.
    ///
    /// # Errors
    ///
    /// Returns [`PointerDataOverTipcError`] when the reply carries pointer data
    /// and the session speaks TIPC, which has no descriptor to put it in.
    pub fn into_reply(self, protocol: Protocol) -> Result<Reply<B>, PointerDataOverTipcError> {
        match protocol {
            Protocol::Cmif(version) => {
                let mut builder = CmifReplyBuilder::new(self.result).with_payload(self.body);
                if let CmifVersion::WithContext { token } = version {
                    builder = builder.with_token(token);
                }
                for &desc in self.send_statics.as_slice() {
                    builder = builder.add_send_static(desc);
                }
                for &handle in self.copy_handles.as_slice() {
                    builder = builder.add_copy_handle(handle);
                }
                for &handle in self.move_handles.as_slice() {
                    builder = builder.add_move_handle(handle);
                }
                Ok(Reply::Cmif(builder.build()))
            }
            Protocol::Tipc => {
                if !self.send_statics.is_empty() {
                    return Err(PointerDataOverTipcError);
                }
                let mut builder = TipcReplyBuilder::new(self.result).with_payload(self.body);
                for &handle in self.copy_handles.as_slice() {
                    builder = builder.add_copy_handle(handle);
                }
                for &handle in self.move_handles.as_slice() {
                    builder = builder.add_move_handle(handle);
                }
                Ok(Reply::Tipc(builder.build()))
            }
        }
    }
}

/// Error returned by [`Response::into_reply`].
///
/// Occurs when the reply carries send statics and the session speaks TIPC,
/// which has no pointer descriptors at all. Reachable only from a handler that
/// returns out-pointer data on an interface a TIPC client reached, so it
/// reports a mismatch between the interface's declared signature and the
/// protocol serving it, not something a client can provoke. Nothing was
/// encoded, so the caller still owes the client a reply.
#[derive(Debug, thiserror::Error)]
#[error("the reply carries pointer data, and TIPC has no descriptor to carry it")]
pub struct PointerDataOverTipcError;

impl ToResultCode for PointerDataOverTipcError {
    fn to_rc(self) -> ResultCode {
        // A reply this crate refused to encode. No service assigned it a code,
        // and the client is owed some answer rather than a dropped session.
        GENERIC_ERROR
    }
}

/// An encoded reply, ready to be written into the thread's IPC buffer.
///
/// Each variant serializes with the `write_to` of the reply type it holds; the
/// server then hands the buffer to the reply-and-receive syscall.
#[derive(Debug, Clone)]
pub enum Reply<B: HipcPayload> {
    /// A CMIF reply, magic header and echoed token included.
    Cmif(CmifReply<B>),
    /// A TIPC reply: a result-code word and the body.
    Tipc(TipcReply<B>),
}

impl<B: HipcPayload> Reply<B> {
    /// Writes the encoded reply into `dst`, whichever protocol carries it.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] when the encoded layout exceeds `N`, leaving
    /// `dst` without a complete reply.
    ///
    /// [`WriteError`]: crate::hipc::WriteError
    pub fn write_to<const N: usize>(
        &self,
        dst: &mut [u8; N],
    ) -> Result<(), crate::hipc::WriteError> {
        match self {
            Self::Cmif(reply) => reply.write_to(dst),
            Self::Tipc(reply) => reply.write_to(dst),
        }
    }
}
