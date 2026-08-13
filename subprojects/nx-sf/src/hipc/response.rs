//! HIPC response messages: parsing one as a client, building one as a server.
//!
//! The two directions are not mirror images. A client parses whatever a server
//! sent and must reject anything malformed ([`parse_response`]); a server
//! builds a reply from values it already holds, and the type it builds through
//! ([`HipcReply`]) has no field for the descriptor kinds a reply may not carry,
//! so the shapes [`parse_response`] rejects cannot be produced here in the
//! first place.

use core::mem::size_of;

use nx_svc::{
    error::ResultCode,
    raw::Handle as RawHandle,
};

use super::wire::{
    HIPC_MAX_DESCRIPTORS,
    Header,
    HipcPayload,
    RECV_LIST_WIRE_NONE,
    SpecialHeader,
    StaticDescriptor,
    WriteError,
    parse_prefix,
    write_section,
};
use crate::{
    array_vec::ArrayVec,
    cursor::{
        Cursor,
        ResponsePayload,
    },
    error::{
        GENERIC_ERROR,
        ToResultCode,
    },
};

/// Message type a server writes into the header of a reply.
///
/// The `message_type` field carries the command type on the way in; on the way
/// back the kernel has already routed the message, so nothing reads it and the
/// wire value is zero. It is the same field [`CommandType::Invalid`] names,
/// which is why a reply cannot be mistaken for a request that lost its type.
///
/// [`CommandType::Invalid`]: crate::cmif::CommandType::Invalid
pub const REPLY_MESSAGE_TYPE: u16 = 0;

/// Parses a full HIPC response into the envelope plus a typed payload.
///
/// Generic over `P: ResponsePayload`: callers pick the payload shape via
/// turbofish — `&T` for a zerocopy struct or `()` for no in-band
/// payload.
///
/// Returns a typed error for any malformed wire shape — never panics on
/// untrusted input. See [`ResponseParseError`] for the failure cases.
///
/// Generic over the buffer size `N`; [`parse_prefix`] enforces at
/// monomorphization that `N >= MIN_PREFIX_BUF_SIZE`.
pub fn parse_response<'a, const N: usize, P>(
    buf: &'a [u8; N],
) -> Result<Response<'a, P>, ResponseParseError>
where
    P: ResponsePayload<'a>,
{
    let envelope = parse_response_envelope(buf)?;
    let cursor = Cursor::new(envelope.data_words);
    let (payload, _) = P::read(cursor).ok_or(ResponseParseError::TruncatedPayload)?;

    Ok(Response {
        payload,
        copy_handles: envelope.copy_handles,
        move_handles: envelope.move_handles,
    })
}

/// Parses the HIPC envelope and exposes the raw data-words region.
///
/// Used by CMIF and TIPC, which build their own cursor over
/// `data_words` to walk their protocol-specific headers before
/// delegating to a [`ResponsePayload`] for the user payload.
pub fn parse_response_envelope<const N: usize>(
    buf: &[u8; N],
) -> Result<Envelope<'_>, ResponseParseError> {
    let (prefix, buf) = parse_prefix(buf);
    let header = &prefix.header;

    if header.num_send_buffers() != 0
        || header.num_recv_buffers() != 0
        || header.num_exch_buffers() != 0
    {
        return Err(ResponseParseError::UnexpectedBufferDescriptor);
    }
    if header.recv_static_mode() != RECV_LIST_WIRE_NONE {
        return Err(ResponseParseError::UnexpectedRecvList);
    }

    let (num_copy_handles, num_move_handles) = match &prefix.extras {
        Some(extras) => (
            extras.num_copy_handles as usize,
            extras.num_move_handles as usize,
        ),
        None => (0, 0),
    };
    let num_statics = header.num_send_statics() as usize;
    let num_data_words = header.num_data_words() as usize;

    // Bound-check the declared payload against the buffer once so the
    // subsequent cursor reads can rely on the fit without re-validating.
    let declared = num_copy_handles * size_of::<RawHandle>()
        + num_move_handles * size_of::<RawHandle>()
        + num_statics * size_of::<StaticDescriptor>()
        + num_data_words * size_of::<u32>();
    if declared > buf.len() {
        return Err(ResponseParseError::DeclaredSizeExceedsBuffer {
            declared,
            capacity: buf.len(),
        });
    }

    // Size check above proves every cursor read below fits.
    let cursor = Cursor::new(buf);
    let (copy_handles, cursor) = cursor
        .read_slice::<RawHandle>(num_copy_handles)
        .expect("internal: size check guarantees fit");
    let (move_handles, cursor) = cursor
        .read_slice::<RawHandle>(num_move_handles)
        .expect("internal: size check guarantees fit");
    let (_statics, cursor) = cursor
        .read_slice::<StaticDescriptor>(num_statics)
        .expect("internal: size check guarantees fit");
    let (data_words, _) = cursor
        .read_bytes(num_data_words * size_of::<u32>())
        .expect("internal: size check guarantees fit");

    Ok(Envelope {
        data_words,
        copy_handles,
        move_handles,
    })
}

/// Error returned by [`parse_response`] and [`parse_response_envelope`].
#[derive(Debug, thiserror::Error)]
pub enum ResponseParseError {
    /// The header's declared descriptor counts imply a message longer than
    /// [`IPC_BUFFER_SIZE`], so the response cannot be decoded without reading
    /// past the end of the TLR buffer.
    #[error("HIPC response declares {declared} bytes but only {capacity} remain in buffer")]
    DeclaredSizeExceedsBuffer {
        /// Total descriptor-region bytes implied by the header's counts.
        declared: usize,
        /// Bytes available after the decoded prefix.
        capacity: usize,
    },
    /// Response carries A/B/W buffer descriptors. These are client→server only;
    /// a server reply must not carry them.
    #[error("HIPC response carries client→server buffer descriptors")]
    UnexpectedBufferDescriptor,
    /// Response declares a Type-C receive list. The receive list is a
    /// request-side construct used by clients to pre-allocate buffers for
    /// server pointer descriptors; it has no meaning in a reply.
    #[error("HIPC response declares a receive-list mode")]
    UnexpectedRecvList,
    /// Data-words region too small to hold the caller-requested payload.
    #[error("HIPC response too small for payload")]
    TruncatedPayload,
}

impl ToResultCode for ResponseParseError {
    fn to_rc(self) -> ResultCode {
        // Every variant is a malformed reply this crate rejected on its own;
        // the server returned success, so it named no code to forward.
        GENERIC_ERROR
    }
}

/// Parsed HIPC response with a typed payload.
///
/// Returned by [`parse_response`]. The payload type is whatever the
/// caller selected via the `P` type parameter.
#[derive(Debug)]
pub struct Response<'a, P> {
    /// In-band payload, parsed from the data-words region.
    pub payload: P,
    /// Copy handles received.
    pub copy_handles: &'a [RawHandle],
    /// Move handles received.
    pub move_handles: &'a [RawHandle],
}

/// Parsed HIPC envelope with the raw data-words region exposed.
///
/// Returned by [`parse_response_envelope`]. CMIF and TIPC consume this
/// shape to walk their own protocol headers before exposing the user
/// payload.
#[derive(Debug)]
pub struct Envelope<'a> {
    /// Raw data-words region, as bytes. The kernel reserves
    /// `num_data_words * 4` bytes here for the protocol payload.
    pub data_words: &'a [u8],
    /// Copy handles received.
    pub copy_handles: &'a [RawHandle],
    /// Move handles received.
    pub move_handles: &'a [RawHandle],
}

/// HIPC reply DTO.
///
/// The server-side counterpart of [`HipcRequest`]: a value-type description of
/// everything a reply puts on the wire: the handles it returns, the send
/// statics carrying out-pointer data, and an in-band payload `P` occupying the
/// data-words region.
///
/// It is a distinct type from [`HipcRequest`] rather than that type with a
/// different message type, because a reply may carry strictly less. A/B/W
/// buffer descriptors are the client's loans of its own memory and the
/// receive list is the client's pre-allocation for pointer data; neither means
/// anything travelling the other way. Giving the reply no field for them is
/// what makes [`ResponseParseError::UnexpectedBufferDescriptor`] and
/// [`ResponseParseError::UnexpectedRecvList`] unreachable from replies this
/// crate builds.
///
/// Serialize via [`write_to`](Self::write_to). Unlike a request, a reply is not
/// sent by a syscall of its own: the server hands the serialized buffer to
/// `ReplyAndReceive`, so nothing here consumes the value.
///
/// [`HipcRequest`]: super::HipcRequest
#[derive(Debug, Clone)]
pub struct HipcReply<P: HipcPayload = ()> {
    send_statics: ArrayVec<StaticDescriptor, HIPC_MAX_DESCRIPTORS>,
    copy_handles: ArrayVec<RawHandle, HIPC_MAX_DESCRIPTORS>,
    move_handles: ArrayVec<RawHandle, HIPC_MAX_DESCRIPTORS>,
    payload: P,
}

impl<P: HipcPayload> HipcReply<P> {
    fn has_special_header(&self) -> bool {
        !self.copy_handles.is_empty() || !self.move_handles.is_empty()
    }

    fn total_bytes(&self, data_words_size: usize) -> usize {
        let mut total = size_of::<Header>();
        if self.has_special_header() {
            total += size_of::<SpecialHeader>();
        }
        total += self.copy_handles.len() * size_of::<RawHandle>();
        total += self.move_handles.len() * size_of::<RawHandle>();
        total += self.send_statics.len() * size_of::<StaticDescriptor>();
        total += data_words_size;
        total
    }

    /// Writes the reply envelope and the payload's data-words region into `dst`.
    ///
    /// The data-words region is sized as `payload.encoded_len()` rounded up to
    /// the next 4-byte boundary, then handed to [`HipcPayload::write_to`] under
    /// that trait's contract; in particular, the region is not pre-zeroed.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] when the encoded layout exceeds `N`, leaving
    /// `dst` without a complete reply.
    pub fn write_to<const N: usize>(&self, dst: &mut [u8; N]) -> Result<(), WriteError> {
        // Round the payload's byte length up to a whole number of 4-byte
        // data words, the unit HIPC headers count in.
        let num_data_words = self.payload.encoded_len().div_ceil(size_of::<u32>());
        let data_words_size = num_data_words * size_of::<u32>();

        let total_bytes = self.total_bytes(data_words_size);
        if total_bytes > N {
            return Err(WriteError {
                needed: total_bytes,
                limit: N,
            });
        }

        let header = Header::new()
            .with_message_type(REPLY_MESSAGE_TYPE)
            // Both casts are lossless: `ArrayVec` caps the descriptor count at
            // `HIPC_MAX_DESCRIPTORS` (15), and the size check above bounds the
            // data-words region by `N`, so the word count cannot exceed `N / 4`.
            .with_num_send_statics(self.send_statics.len() as u8)
            .with_num_data_words(num_data_words as u16)
            .with_recv_static_mode(RECV_LIST_WIRE_NONE)
            .with_has_special_header(self.has_special_header());

        let buf = write_section(&mut dst[..], &header);
        let buf = if self.has_special_header() {
            // Lossless for the same reason as the header counts above: both
            // vectors cap at `HIPC_MAX_DESCRIPTORS`.
            let special = SpecialHeader::new()
                .with_send_pid(false)
                .with_num_copy_handles(self.copy_handles.len() as u8)
                .with_num_move_handles(self.move_handles.len() as u8);
            write_section(buf, &special)
        } else {
            buf
        };
        let buf = write_section(buf, &self.copy_handles[..]);
        let buf = write_section(buf, &self.move_handles[..]);
        let buf = write_section(buf, &self.send_statics[..]);
        let (data_words, _) = buf.split_at_mut(data_words_size);
        self.payload.write_to(data_words);

        Ok(())
    }
}

/// Fluent builder for a [`HipcReply`].
///
/// Mirrors the request builder on the reply side, minus every accumulator a
/// reply has no wire slot for. The in-band payload is attached via
/// [`with_payload`](Self::with_payload), which type-changes the builder.
/// Finalize via [`build`](Self::build).
pub struct HipcReplyBuilder<P: HipcPayload = ()> {
    send_statics: ArrayVec<StaticDescriptor, HIPC_MAX_DESCRIPTORS>,
    copy_handles: ArrayVec<RawHandle, HIPC_MAX_DESCRIPTORS>,
    move_handles: ArrayVec<RawHandle, HIPC_MAX_DESCRIPTORS>,
    payload: P,
}

impl HipcReplyBuilder {
    /// Starts a new builder for a reply with no in-band payload.
    ///
    /// Attach one via [`with_payload`](Self::with_payload).
    #[inline]
    pub fn new() -> Self {
        Self {
            send_statics: Default::default(),
            copy_handles: Default::default(),
            move_handles: Default::default(),
            payload: (),
        }
    }
}

impl Default for HipcReplyBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<P: HipcPayload> HipcReplyBuilder<P> {
    /// Attaches an in-band payload, type-changing the builder to carry `Q`.
    ///
    /// All previously-accumulated envelope state is preserved.
    #[inline]
    pub fn with_payload<Q: HipcPayload>(self, payload: Q) -> HipcReplyBuilder<Q> {
        HipcReplyBuilder {
            send_statics: self.send_statics,
            copy_handles: self.copy_handles,
            move_handles: self.move_handles,
            payload,
        }
    }

    /// Appends a send-static (Type X) descriptor carrying returned pointer data.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if more than [`HIPC_MAX_DESCRIPTORS`] entries are
    /// added; the wire-format cap is hardware-fixed.
    #[inline]
    pub fn with_send_static(mut self, desc: StaticDescriptor) -> Self {
        self.send_statics.push(desc);
        self
    }

    /// Appends a copy handle, which the kernel duplicates into the client.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if more than [`HIPC_MAX_DESCRIPTORS`] entries are
    /// added; the wire-format cap is hardware-fixed.
    #[inline]
    pub fn with_copy_handle(mut self, handle: RawHandle) -> Self {
        self.copy_handles.push(handle);
        self
    }

    /// Appends a move handle, transferring ownership to the client.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if more than [`HIPC_MAX_DESCRIPTORS`] entries are
    /// added; the wire-format cap is hardware-fixed.
    #[inline]
    pub fn with_move_handle(mut self, handle: RawHandle) -> Self {
        self.move_handles.push(handle);
        self
    }

    /// Finalizes the builder into a [`HipcReply`] DTO.
    pub fn build(self) -> HipcReply<P> {
        HipcReply {
            send_statics: self.send_statics,
            copy_handles: self.copy_handles,
            move_handles: self.move_handles,
            payload: self.payload,
        }
    }
}
