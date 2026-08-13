//! HIPC request messages: building one as a client, parsing one as a server.
//!
//! The build path ([`HipcRequestBuilder`] → [`HipcRequest`]) puts a request on
//! the wire; the parse path ([`parse_request`] → [`Request`]) reads one the
//! kernel delivered. They are not inverses of each other and do not share code
//! beyond the wire-format types: the builder emits descriptors it derives from
//! loans it holds, whereas the parser hands back borrows into a buffer whose
//! bytes an untrusted client chose.
//!
//! # Trust boundary
//!
//! Every field in an inbound request is attacker-controlled, including the
//! descriptor counts that say how long the message is. [`parse_request`]
//! therefore validates the declared layout against the buffer before reading a
//! single section, and returns a typed error for every shape it rejects; it has
//! no panicking path on untrusted input. What it does **not** do is vouch for
//! the descriptor *targets*: a buffer descriptor's address and size are numbers
//! the sender picked, and only the kernel's mapping makes them real. See
//! [`Request`] for what that means for a caller.
//!
//! # DTO model
//!
//! [`HipcRequestBuilder`] accumulates HIPC-level descriptors (statics, buffers,
//! handles, recv-list) and finalizes into a [`HipcRequest`] DTO via
//! [`HipcRequestBuilder::build`]. The DTO carries every input needed to
//! serialize the request - descriptors, handles, recv-list configuration,
//! send-PID flag, and the in-band payload `P`.
//!
//! # In-band payloads
//!
//! [`HipcRequest`] is parametric over an in-band payload type `P: HipcPayload`.
//! The payload owns the bytes that go into the data-words region. HIPC asks
//! the payload for its [`encoded_len`](HipcPayload::encoded_len), pads up to
//! the 4-byte word boundary, writes the envelope, zero-fills the region, and
//! finally delegates to [`HipcPayload::write_to`].
//!
//! Built-in payloads cover the common cases:
//!
//! - `()` - no in-band data; the data-words region is empty.
//! - `&[u8]` - raw byte slice copied verbatim.
//!
//! Higher-level protocols (CMIF, TIPC) define their own payload types and
//! implement [`HipcPayload`] for them, so HIPC drives the whole write without
//! reaching back into the data region after the fact.
//!
//! Descriptor counts are bounded by the HIPC header's 4-bit fields, so the
//! DTO uses inline `[T; HIPC_MAX_DESCRIPTORS]` storage - no heap, no dynamic
//! allocation.

use core::mem::size_of;

use nx_svc::{
    error::{
        ResultCode,
        ToResultCode as _,
    },
    ipc::SendSyncError,
    raw::Handle as RawHandle,
};
use nx_sys_thread_tls::IpcBuffer;

use super::wire::{
    BufferDescriptor,
    Header,
    MessageType,
    ProcessId,
    RECV_LIST_WIRE_NONE,
    RECV_LIST_WIRE_SINGLE_BUFFER,
    RECV_LIST_WIRE_TO_MESSAGE_BUFFER,
    RecvListEntry,
    SpecialHeader,
    StaticDescriptor,
    parse_prefix,
    write_section,
};
use crate::{
    array_vec::ArrayVec,
    cursor::Cursor,
    error::{
        GENERIC_ERROR,
        ToResultCode,
    },
    service::handle::BorrowedSessionHandle,
};

/// Maximum descriptors of any single kind that fit in an HIPC header
/// (each `num_*` field is 4 bits wide).
pub const HIPC_MAX_DESCRIPTORS: usize = 15;

/// Maximum receive-list entries. Per-pointer mode encodes `2 + n` in the
/// 4-bit `recv_static_mode` field, capping `n` at 13.
pub const HIPC_MAX_RECV_LIST: usize = 13;

/// HIPC request DTO.
///
/// Self-contained value-type description of an HIPC request: message type,
/// every descriptor / handle that goes on the wire, recv-list configuration,
/// `send_pid` flag, and an in-band payload `P`. Constructed by
/// [`HipcRequestBuilder::build`].
///
/// Serialize via [`write_to`](Self::write_to), which writes the HIPC envelope
/// and then delegates the data-words region to the payload encoder.
#[derive(Debug, Clone)]
pub struct HipcRequest<P: HipcPayload = ()> {
    message_type: MessageType,
    send_statics: ArrayVec<StaticDescriptor, HIPC_MAX_DESCRIPTORS>,
    send_buffers: ArrayVec<BufferDescriptor, HIPC_MAX_DESCRIPTORS>,
    recv_buffers: ArrayVec<BufferDescriptor, HIPC_MAX_DESCRIPTORS>,
    exch_buffers: ArrayVec<BufferDescriptor, HIPC_MAX_DESCRIPTORS>,
    copy_handles: ArrayVec<RawHandle, HIPC_MAX_DESCRIPTORS>,
    move_handles: ArrayVec<RawHandle, HIPC_MAX_DESCRIPTORS>,
    recv_list_mode: RecvListMode,
    send_pid: bool,
    payload: P,
}

impl<P: HipcPayload> HipcRequest<P> {
    fn has_special_header(&self) -> bool {
        self.send_pid || !self.copy_handles.is_empty() || !self.move_handles.is_empty()
    }

    fn total_bytes(&self, data_words_size: usize) -> usize {
        let mut total = size_of::<Header>();
        if self.has_special_header() {
            total += size_of::<SpecialHeader>();
            if self.send_pid {
                total += size_of::<u64>();
            }
        }
        total += self.copy_handles.len() * size_of::<RawHandle>();
        total += self.move_handles.len() * size_of::<RawHandle>();
        total += self.send_statics.len() * size_of::<StaticDescriptor>();
        total += self.send_buffers.len() * size_of::<BufferDescriptor>();
        total += self.recv_buffers.len() * size_of::<BufferDescriptor>();
        total += self.exch_buffers.len() * size_of::<BufferDescriptor>();
        total += data_words_size;
        total += self.recv_list_mode.wire_slot_count() * size_of::<RecvListEntry>();
        total
    }

    /// Writes the HIPC envelope and the payload's data-words region into `dst`.
    ///
    /// The data-words region is sized as `payload.encoded_len()` rounded up to
    /// the next 4-byte boundary, zero-filled, then handed to
    /// [`HipcPayload::write_to`]. The total layout size must fit in `N`;
    /// otherwise [`WriteError`] is returned.
    pub(crate) fn write_to<const N: usize>(&self, dst: &mut [u8; N]) -> Result<(), WriteError> {
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

        let buf = &mut dst[..];
        let buf = write_header(buf, self, num_data_words);
        let buf = if self.has_special_header() {
            write_special_header(buf, self)
        } else {
            buf
        };
        let buf = write_section(buf, &self.copy_handles[..]);
        let buf = write_section(buf, &self.move_handles[..]);
        let buf = write_section(buf, &self.send_statics[..]);
        let buf = write_section(buf, &self.send_buffers[..]);
        let buf = write_section(buf, &self.recv_buffers[..]);
        let buf = write_section(buf, &self.exch_buffers[..]);
        let buf = write_data_words(buf, &self.payload, data_words_size);
        let _ = write_recv_list(buf, &self.recv_list_mode);

        Ok(())
    }

    /// Serializes the request into `buf` and issues the synchronous IPC
    /// syscall on `session`.
    ///
    /// Consuming `self` keeps every descriptor loan attached to the request
    /// alive across the syscall: the kernel reads and writes the loaned
    /// buffers while this function runs, and the borrows are released only
    /// when it returns. This is the sole path from a request value to the
    /// kernel - the serialize-then-send sequence is indivisible, so the
    /// descriptor bytes in the TLS buffer can never outlive the loans that
    /// justify them.
    pub(crate) fn send_inner(
        self,
        buf: &mut IpcBuffer,
        session: BorrowedSessionHandle<'_>,
    ) -> Result<(), SendError> {
        self.write_to(buf.as_array_mut())
            .map_err(SendError::Layout)?;
        // SAFETY: every descriptor just serialized into `buf` derives from a
        // loan held in `self`, which lives until this function returns -
        // after the syscall has completed.
        unsafe { crate::ipc::send_sync_request(buf, session) }.map_err(SendError::SendRequest)
        // `self` - and with it every buffer loan - drops here.
    }
}

/// Error returned by [`HipcRequest::write_to`] when the destination buffer
/// is too small to hold the encoded request.
///
/// HIPC request layout is computed from the accumulated envelope
/// (descriptors, handles, recv-list, optional special header) plus the
/// payload's data-words region (sized as
/// `payload.encoded_len().next_multiple_of(4)`). If that total exceeds the
/// caller-supplied destination buffer's `N` bytes,
/// [`write_to`](HipcRequest::write_to) returns this error instead of
/// writing a partial request. The fields report the layout requirement
/// and the buffer capacity so callers can either size their IPC buffer
/// to fit or drop descriptors/payload to fit the available space.
///
/// Building a [`HipcRequest`] is infallible - this error only surfaces at
/// serialization time, when the destination buffer is known.
#[derive(Debug, thiserror::Error)]
#[error("request layout requires {needed} bytes, IPC buffer holds {limit}")]
pub struct WriteError {
    /// Total bytes the encoded request layout requires.
    pub needed: usize,
    /// Capacity of the destination buffer.
    pub limit: usize,
}

impl ToResultCode for WriteError {
    fn to_rc(self) -> ResultCode {
        // Caught before the syscall, so no server saw the request and there is
        // no service code to forward.
        GENERIC_ERROR
    }
}

/// Error returned by [`HipcRequest::send_inner`] and the protocol-level
/// `send` methods built on it (CMIF and TIPC).
#[derive(Debug, thiserror::Error)]
pub enum SendError {
    /// The encoded request layout does not fit in the IPC buffer.
    ///
    /// Detected before the syscall is issued; nothing was sent and the
    /// TLS buffer holds no complete request.
    #[error("request layout exceeds the IPC buffer")]
    Layout(#[source] WriteError),
    /// The kernel rejected the underlying `SendSyncRequest`.
    ///
    /// The request was serialized and the syscall was issued; output
    /// buffer contents are unspecified (any bit pattern, still valid
    /// `u8`s).
    #[error("failed to send the IPC request")]
    SendRequest(#[source] SendSyncError),
}

impl ToResultCode for SendError {
    fn to_rc(self) -> ResultCode {
        match self {
            SendError::Layout(err) => err.to_rc(),
            // The kernel owns this code, so it resolves through `nx-svc`'s
            // trait rather than this crate's.
            SendError::SendRequest(err) => err.to_rc(),
        }
    }
}

/// Fluent builder for an [`HipcRequest`].
///
/// Accumulates HIPC-level descriptors via `with_*` methods. The in-band
/// payload is attached via [`with_payload`](Self::with_payload), which
/// type-changes the builder from `HipcRequestBuilder<P>` to
/// `HipcRequestBuilder<Q>`. Finalize via [`build`](Self::build).
pub(crate) struct HipcRequestBuilder<P: HipcPayload = ()> {
    message_type: MessageType,
    send_statics: ArrayVec<StaticDescriptor, HIPC_MAX_DESCRIPTORS>,
    send_buffers: ArrayVec<BufferDescriptor, HIPC_MAX_DESCRIPTORS>,
    recv_buffers: ArrayVec<BufferDescriptor, HIPC_MAX_DESCRIPTORS>,
    exch_buffers: ArrayVec<BufferDescriptor, HIPC_MAX_DESCRIPTORS>,
    copy_handles: ArrayVec<RawHandle, HIPC_MAX_DESCRIPTORS>,
    move_handles: ArrayVec<RawHandle, HIPC_MAX_DESCRIPTORS>,
    recv_list_mode: RecvListMode,
    send_pid: bool,
    payload: P,
}

impl HipcRequestBuilder {
    /// Starts a new builder for the given message type with no in-band payload.
    ///
    /// `message_type` accepts any value convertible into [`MessageType`] -
    /// typically a CMIF or TIPC `CommandType`. Attach a payload via
    /// [`with_payload`](Self::with_payload).
    pub fn new(message_type: impl Into<MessageType>) -> Self {
        Self {
            message_type: message_type.into(),
            send_statics: Default::default(),
            send_buffers: Default::default(),
            recv_buffers: Default::default(),
            exch_buffers: Default::default(),
            copy_handles: Default::default(),
            move_handles: Default::default(),
            recv_list_mode: RecvListMode::None,
            send_pid: false,
            payload: (),
        }
    }
}

impl<P: HipcPayload> HipcRequestBuilder<P> {
    /// Replaces the message type chosen at construction.
    ///
    /// CMIF callers use this to switch between
    /// [`Request`](crate::cmif::CommandType::Request) and
    /// [`RequestWithContext`](crate::cmif::CommandType::RequestWithContext)
    /// once a context token has been recorded.
    #[inline]
    pub fn set_message_type(mut self, message_type: impl Into<MessageType>) -> Self {
        self.message_type = message_type.into();
        self
    }

    /// Enables sending the process ID alongside the request.
    #[inline]
    pub fn with_send_pid(mut self) -> Self {
        self.send_pid = true;
        self
    }

    /// Attaches an in-band payload, type-changing the builder to carry `Q`.
    ///
    /// All previously-accumulated envelope state (descriptors, handles,
    /// recv-list, `send_pid`) is preserved. Subsequent `with_*` calls operate
    /// on the new builder.
    #[inline]
    pub fn with_payload<Q: HipcPayload>(self, payload: Q) -> HipcRequestBuilder<Q> {
        HipcRequestBuilder {
            message_type: self.message_type,
            send_statics: self.send_statics,
            send_buffers: self.send_buffers,
            recv_buffers: self.recv_buffers,
            exch_buffers: self.exch_buffers,
            copy_handles: self.copy_handles,
            move_handles: self.move_handles,
            recv_list_mode: self.recv_list_mode,
            send_pid: self.send_pid,
            payload,
        }
    }

    /// Appends a send-static (Type X) descriptor.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if more than [`HIPC_MAX_DESCRIPTORS`] entries
    /// are added; the wire-format cap is hardware-fixed.
    #[inline]
    pub fn with_send_static(mut self, desc: StaticDescriptor) -> Self {
        self.send_statics.push(desc);
        self
    }

    /// Appends a send-buffer (Type A) descriptor.
    #[inline]
    pub fn with_send_buffer(mut self, desc: BufferDescriptor) -> Self {
        self.send_buffers.push(desc);
        self
    }

    /// Appends a receive-buffer (Type B) descriptor.
    #[inline]
    pub fn with_recv_buffer(mut self, desc: BufferDescriptor) -> Self {
        self.recv_buffers.push(desc);
        self
    }

    /// Appends an exchange-buffer (Type W) descriptor.
    #[inline]
    pub fn with_exch_buffer(mut self, desc: BufferDescriptor) -> Self {
        self.exch_buffers.push(desc);
        self
    }

    /// Appends a copy handle slot.
    #[inline]
    pub fn with_copy_handle(mut self, handle: RawHandle) -> Self {
        self.copy_handles.push(handle);
        self
    }

    /// Appends a move handle slot.
    #[inline]
    pub fn with_move_handle(mut self, handle: RawHandle) -> Self {
        self.move_handles.push(handle);
        self
    }

    /// Appends a per-pointer recv-list entry, transitioning the builder into
    /// [`RecvListMode::Entries`] (wire mode `2 + n`).
    #[inline]
    pub fn with_recv_list_entry(mut self, entry: RecvListEntry) -> Self {
        match &mut self.recv_list_mode {
            RecvListMode::Entries(v) => v.push(entry),
            RecvListMode::None => {
                let mut v = ArrayVec::new();
                v.push(entry);
                self.recv_list_mode = RecvListMode::Entries(v);
            }
        }
        self
    }

    /// Finalizes the builder into a [`HipcRequest`] DTO.
    pub fn build(self) -> HipcRequest<P> {
        HipcRequest {
            message_type: self.message_type,
            send_statics: self.send_statics,
            send_buffers: self.send_buffers,
            recv_buffers: self.recv_buffers,
            exch_buffers: self.exch_buffers,
            copy_handles: self.copy_handles,
            move_handles: self.move_handles,
            recv_list_mode: self.recv_list_mode,
            send_pid: self.send_pid,
            payload: self.payload,
        }
    }
}

/// Encoder for the in-band data-words region of an HIPC request.
///
/// HIPC owns the envelope (header, descriptors, handles); the payload owns
/// everything that goes into the data-words region. Higher-level protocols
/// (CMIF, TIPC) implement this trait for their wire-format bodies and attach
/// them to a [`HipcRequest`] via [`HipcRequestBuilder::with_payload`].
///
/// # Contract
///
/// [`HipcRequest::write_to`] computes the data-words region as
/// `encoded_len().next_multiple_of(4)` and hands the impl a `dst` slice of
/// exactly that length. The region is **not** pre-zeroed - IPC is on the
/// hot path, and the previous global fill duplicated writes the impl
/// already performs for its sections. Bytes in `dst` that the impl does
/// not overwrite (alignment slack, trailing word padding) are transmitted
/// as-is from the caller's TLS buffer; well-behaved servers parse by
/// structure layout and ignore them. Impls that need deterministic wire
/// bytes must zero those regions themselves. Encoding is infallible - the
/// destination slice is guaranteed large enough by construction, and
/// CMIF/TIPC wire-format bodies have no other failure modes.
pub trait HipcPayload {
    /// Byte length of the encoded payload, **unrounded**.
    ///
    /// HIPC rounds this up to the next 4-byte word boundary when sizing the
    /// data-words region.
    fn encoded_len(&self) -> usize;

    /// Writes the payload into the data-words region starting at `dst[0]`.
    ///
    /// `dst.len()` equals [`encoded_len`](Self::encoded_len) rounded up to a
    /// 4-byte multiple. The region is **not** pre-zeroed; see the trait-level
    /// [`Contract`](Self#contract) for the rules governing untouched bytes.
    fn write_to(&self, dst: &mut [u8]);
}

impl HipcPayload for () {
    #[inline]
    fn encoded_len(&self) -> usize {
        0
    }

    #[inline]
    fn write_to(&self, _: &mut [u8]) {}
}

impl HipcPayload for &[u8] {
    #[inline]
    fn encoded_len(&self) -> usize {
        self.len()
    }

    #[inline]
    fn write_to(&self, dst: &mut [u8]) {
        dst[..self.len()].copy_from_slice(self);
    }
}

/// Receive-list (Type C) configuration for the HIPC header.
///
/// Lowered to the 4-bit `recv_static_mode` field on the build path - see
/// [`RECV_LIST_WIRE_NONE`] in the wire module for the authoritative
/// wire-encoding table. Only the two client-side shapes are modeled; the
/// wire-legal "to message buffer" (mode `1`) and "single buffer" (mode `2`)
/// shapes are server-side receive patterns with no client use.
///
/// The default is [`RecvListMode::None`].
#[derive(Debug, Default, Clone)]
pub(crate) enum RecvListMode {
    /// No recv-list; the server may not return Type-X pointer data (wire mode
    /// `0`).
    #[default]
    None,
    /// Per-pointer recv-list: entry `i` is the destination of the `i`-th
    /// out-pointer descriptor (wire mode `2 + n`, `n = entries.len()`).
    ///
    /// Constructed exclusively via [`HipcRequestBuilder::with_recv_list_entry`]
    /// so the variant is never observably empty.
    Entries(ArrayVec<RecvListEntry, HIPC_MAX_RECV_LIST>),
}

impl RecvListMode {
    /// Number of recv-list slots this mode reserves on the wire.
    #[inline]
    pub fn wire_slot_count(&self) -> usize {
        match self {
            RecvListMode::None => 0,
            RecvListMode::Entries(v) => v.len(),
        }
    }

    /// Encodes the 4-bit `recv_static_mode` wire field.
    #[inline]
    pub fn to_raw(&self) -> u8 {
        match self {
            RecvListMode::None => RECV_LIST_WIRE_NONE,
            RecvListMode::Entries(v) => RECV_LIST_WIRE_SINGLE_BUFFER + v.len() as u8,
        }
    }
}

/// Parses an inbound HIPC request message.
///
/// This is the server-side entry point: `buf` holds the message the kernel
/// delivered into the thread's IPC buffer, and every byte of it was chosen by
/// the sending client. The returned [`Request`] borrows `buf` in place — no
/// copies, no allocation.
///
/// Generic over the buffer size `N`, which must be at least large enough for
/// the worst-case wire prefix; the prefix decoder enforces that at
/// monomorphization, so a too-small buffer is a compile error rather than a
/// runtime one.
///
/// # Errors
///
/// Returns [`RequestParseError`] for any malformed wire shape. It never panics
/// on untrusted input: the declared layout is bound-checked against the buffer
/// before any section is read.
pub fn parse_request<const N: usize>(buf: &[u8; N]) -> Result<Request<'_>, RequestParseError> {
    let (prefix, rest) = parse_prefix(buf);
    let header = &prefix.header;

    let (num_copy_handles, num_move_handles, process_id) = match prefix.extras {
        Some(extras) => (
            extras.num_copy_handles as usize,
            extras.num_move_handles as usize,
            extras.pid,
        ),
        None => (0, 0, None),
    };

    let num_send_statics = header.num_send_statics() as usize;
    let num_send_buffers = header.num_send_buffers() as usize;
    let num_recv_buffers = header.num_recv_buffers() as usize;
    let num_exch_buffers = header.num_exch_buffers() as usize;
    let num_data_words = header.num_data_words() as usize;
    let recv_static_mode = header.recv_static_mode();
    let num_recv_list = recv_list_slot_count(recv_static_mode);

    // Bound-check the whole declared layout against the buffer once, so the
    // section reads below cannot run past the end whatever the sender declared.
    let declared = (num_copy_handles + num_move_handles) * size_of::<RawHandle>()
        + num_send_statics * size_of::<StaticDescriptor>()
        + (num_send_buffers + num_recv_buffers + num_exch_buffers) * size_of::<BufferDescriptor>()
        + num_data_words * size_of::<u32>()
        + num_recv_list * size_of::<RecvListEntry>();
    if declared > rest.len() {
        return Err(RequestParseError::DeclaredSizeExceedsBuffer {
            declared,
            capacity: rest.len(),
        });
    }

    // The size check above leaves alignment as the only way a read can fail,
    // and that is a property of the buffer the caller supplied rather than of
    // anything the sender wrote.
    let cursor = Cursor::new(rest);
    let (copy_handles, cursor) = cursor
        .read_slice::<RawHandle>(num_copy_handles)
        .ok_or(RequestParseError::UnalignedBuffer)?;
    let (move_handles, cursor) = cursor
        .read_slice::<RawHandle>(num_move_handles)
        .ok_or(RequestParseError::UnalignedBuffer)?;
    let (send_statics, cursor) = cursor
        .read_slice::<StaticDescriptor>(num_send_statics)
        .ok_or(RequestParseError::UnalignedBuffer)?;
    let (send_buffers, cursor) = cursor
        .read_slice::<BufferDescriptor>(num_send_buffers)
        .ok_or(RequestParseError::UnalignedBuffer)?;
    let (recv_buffers, cursor) = cursor
        .read_slice::<BufferDescriptor>(num_recv_buffers)
        .ok_or(RequestParseError::UnalignedBuffer)?;
    let (exch_buffers, cursor) = cursor
        .read_slice::<BufferDescriptor>(num_exch_buffers)
        .ok_or(RequestParseError::UnalignedBuffer)?;
    let (data_words, cursor) = cursor
        .read_bytes(num_data_words * size_of::<u32>())
        .ok_or(RequestParseError::UnalignedBuffer)?;
    let (recv_list_entries, _) = cursor
        .read_slice::<RecvListEntry>(num_recv_list)
        .ok_or(RequestParseError::UnalignedBuffer)?;

    Ok(Request {
        message_type: MessageType::from_raw(header.message_type()),
        process_id,
        copy_handles,
        move_handles,
        send_statics,
        send_buffers,
        recv_buffers,
        exch_buffers,
        data_words,
        recv_list: classify_recv_list(recv_static_mode, recv_list_entries),
    })
}

/// Error returned by [`parse_request`].
#[derive(Debug, thiserror::Error)]
pub enum RequestParseError {
    /// The header's declared descriptor counts imply a message longer than the
    /// supplied buffer, so the request cannot be decoded without reading past
    /// its end.
    ///
    /// The counts come from the sender, so this is the expected shape of a
    /// malformed or hostile request rather than an internal inconsistency.
    #[error("HIPC request declares {declared} bytes but only {capacity} remain in buffer")]
    DeclaredSizeExceedsBuffer {
        /// Total descriptor-region bytes implied by the header's counts.
        declared: usize,
        /// Bytes available after the decoded prefix.
        capacity: usize,
    },
    /// The supplied buffer's base address is not 4-byte aligned, so the
    /// fixed-layout sections cannot be borrowed in place.
    ///
    /// A property of the caller's buffer, not of the message: the thread's IPC
    /// buffer satisfies this, and nothing a sender writes can cause it.
    #[error("HIPC request buffer is not 4-byte aligned")]
    UnalignedBuffer,
}

#[cfg(feature = "ffi")]
impl crate::error::ToResultCode for RequestParseError {
    fn to_rc(self) -> crate::error::ResultCode {
        // A request this crate rejected before any handler saw it. No service
        // assigned it a code, so there is nothing to forward.
        crate::error::GENERIC_ERROR
    }
}

/// Parsed inbound HIPC request.
///
/// Returned by [`parse_request`]. Every field borrows the caller's buffer, so
/// the value lives exactly as long as the message it describes — which is what
/// keeps a reply, written into the same buffer, from being built from bytes it
/// has already overwritten.
///
/// # Descriptor targets are not validated
///
/// The descriptor slices report what the sender *declared*. A
/// [`BufferDescriptor`]'s address and size are numbers from the client, made
/// real only by the mapping the kernel established for the duration of the
/// request; a [`StaticDescriptor`]'s bytes were copied by the kernel into the
/// server's pointer buffer. Reading through either one means constructing a
/// slice from a raw address, which is why this type hands back the descriptors
/// themselves rather than slices: the step that vouches for a target belongs to
/// the layer that knows which mapping is live, not to a parser.
#[derive(Debug)]
pub struct Request<'a> {
    /// Message type field, carrying the protocol's command type.
    pub message_type: MessageType,
    /// Sender's process ID, present when the client set the `send_pid` bit.
    ///
    /// The kernel writes this slot itself, so it identifies the sender rather
    /// than repeating a claim the sender made.
    pub process_id: Option<ProcessId>,
    /// Handles the kernel duplicated into this process.
    pub copy_handles: &'a [RawHandle],
    /// Handles whose ownership transferred to this process.
    pub move_handles: &'a [RawHandle],
    /// Type-X send statics: pointer data the kernel copied in.
    pub send_statics: &'a [StaticDescriptor],
    /// Type-A send buffers: client memory mapped read-only.
    pub send_buffers: &'a [BufferDescriptor],
    /// Type-B receive buffers: client memory mapped read-write, for output.
    pub recv_buffers: &'a [BufferDescriptor],
    /// Type-W exchange buffers: client memory mapped read-write, bidirectional.
    pub exch_buffers: &'a [BufferDescriptor],
    /// Raw data-words region, as bytes. The protocol layer (CMIF, TIPC) parses
    /// its own headers out of this.
    pub data_words: &'a [u8],
    /// Type-C receive list: where returned pointer data is to be written.
    pub recv_list: RecvList<'a>,
}

/// Type-C receive list declared by an inbound request.
///
/// The four wire modes differ in how many slots they reserve and what the
/// server may do with them, so each is its own variant: a caller that matches
/// on this cannot read entries a mode did not reserve, nor treat "write into
/// the message buffer" as if it named a destination.
#[derive(Debug, Clone, Copy)]
pub enum RecvList<'a> {
    /// Wire mode `0`: no receive list. The server may not return pointer data.
    None,
    /// Wire mode `1`: returned pointer data goes into the client's TLS message
    /// buffer, so the client reserved no slot to name a destination.
    ToMessageBuffer,
    /// Wire mode `2`: a single destination the server may subdivide across all
    /// the pointer data it returns.
    SingleBuffer(&'a RecvListEntry),
    /// Wire mode `2 + n`: one destination per returned pointer, entry `i`
    /// taking the `i`-th out-pointer.
    Entries(&'a [RecvListEntry]),
}

/// Number of receive-list slots a `recv_static_mode` wire value reserves.
///
/// See [`RECV_LIST_WIRE_NONE`] for the encoding table.
#[inline]
fn recv_list_slot_count(mode: u8) -> usize {
    match mode {
        RECV_LIST_WIRE_NONE | RECV_LIST_WIRE_TO_MESSAGE_BUFFER => 0,
        RECV_LIST_WIRE_SINGLE_BUFFER => 1,
        // The arms above take every value below `RECV_LIST_WIRE_SINGLE_BUFFER`,
        // so the subtraction cannot wrap; the field is 4 bits wide, capping the
        // result at 13.
        n => usize::from(n - RECV_LIST_WIRE_SINGLE_BUFFER),
    }
}

/// Pairs a `recv_static_mode` wire value with the entries it reserved.
///
/// Total by construction: [`recv_list_slot_count`] sized `entries`, so the
/// single-buffer arm's one-element pattern is the only shape that mode can
/// produce.
#[inline]
fn classify_recv_list(mode: u8, entries: &[RecvListEntry]) -> RecvList<'_> {
    match (mode, entries) {
        (RECV_LIST_WIRE_NONE, _) => RecvList::None,
        (RECV_LIST_WIRE_TO_MESSAGE_BUFFER, _) => RecvList::ToMessageBuffer,
        (RECV_LIST_WIRE_SINGLE_BUFFER, [entry]) => RecvList::SingleBuffer(entry),
        _ => RecvList::Entries(entries),
    }
}

/// Writes the HIPC header into `buf` and returns the remaining tail.
fn write_header<'a, P: HipcPayload>(
    buf: &'a mut [u8],
    request: &HipcRequest<P>,
    num_data_words: usize,
) -> &'a mut [u8] {
    let header = Header::new()
        .with_message_type(request.message_type.to_raw())
        .with_num_send_statics(request.send_statics.len() as u8)
        .with_num_send_buffers(request.send_buffers.len() as u8)
        .with_num_recv_buffers(request.recv_buffers.len() as u8)
        .with_num_exch_buffers(request.exch_buffers.len() as u8)
        .with_num_data_words(num_data_words as u16)
        .with_recv_static_mode(request.recv_list_mode.to_raw())
        .with_has_special_header(request.has_special_header());

    write_section(buf, &header)
}

/// Writes the special header and the optional PID slot.
///
/// Only called when [`HipcRequest::has_special_header`] is true.
fn write_special_header<'a, P: HipcPayload>(
    buf: &'a mut [u8],
    request: &HipcRequest<P>,
) -> &'a mut [u8] {
    let special = SpecialHeader::new()
        .with_send_pid(request.send_pid)
        .with_num_copy_handles(request.copy_handles.len() as u8)
        .with_num_move_handles(request.move_handles.len() as u8);

    let buf = write_section(buf, &special);

    if request.send_pid {
        // Reserved PID slot - the kernel fills it on transmission.
        let (_, rest) = buf.split_at_mut(size_of::<u64>());
        rest
    } else {
        buf
    }
}

/// Hands the data-words region to the payload for serialization.
///
/// The region is **not** pre-zeroed: IPC sits on the hot path and the global
/// fill duplicates writes already performed by the payload's section emits.
/// Per the [`HipcPayload`] contract, any padding bytes the impl leaves
/// untouched are transmitted as-is from the caller's TLS buffer.
fn write_data_words<'a, P: HipcPayload>(
    buf: &'a mut [u8],
    payload: &P,
    data_words_size: usize,
) -> &'a mut [u8] {
    let (buf, tail) = buf.split_at_mut(data_words_size);
    payload.write_to(buf);
    tail
}

/// Writes the recv-list section per the configured [`RecvListMode`].
fn write_recv_list<'a>(buf: &'a mut [u8], mode: &RecvListMode) -> &'a mut [u8] {
    match mode {
        RecvListMode::Entries(v) => write_section(buf, v.as_slice()),
        RecvListMode::None => buf,
    }
}
