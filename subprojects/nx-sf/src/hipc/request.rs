//! HIPC request building (client side).
//!
//! Only the build path is implemented in this crate — see the parent module's
//! "Server-side request parsing" section for why a `parse_request` counterpart
//! is intentionally absent.
//!
//! # Builder model
//!
//! [`HipcRequestBuilder`] accumulates HIPC-level descriptors (statics, buffers,
//! handles, recv-list) without holding any buffer reference. Once
//! protocol-specific contents are known the caller invokes
//! [`HipcRequestBuilder::payload`] with the destination `&mut [u8; N]` buffer
//! and a [`HipcPayload`] writer (CMIF, TIPC, …); `payload` computes the final
//! layout, writes the HIPC header and all descriptor slots via zerocopy, then
//! hands the carved data-words region to the writer to fill. The writer's
//! [`Output`](HipcPayload::Output) is the protocol-shaped value returned to the
//! caller and is the sole borrower of the supplied buffer.
//!
//! Descriptor counts are bounded by the HIPC header's 4-bit fields, so the
//! builder uses inline `[T; HIPC_MAX_DESCRIPTORS]` storage — no heap, no
//! dynamic allocation.

use core::mem::size_of;

use nx_svc::raw::Handle as RawHandle;
use zerocopy::FromBytes;

use super::{
    array_vec::ArrayVec,
    wire::{
        BufferDescriptor, Header, MessageType, RECV_LIST_WIRE_NONE, RECV_LIST_WIRE_SINGLE_BUFFER,
        RECV_LIST_WIRE_TO_MESSAGE_BUFFER, RecvListEntry, SpecialHeader, StaticDescriptor,
    },
};

/// Maximum descriptors of any single kind that fit in an HIPC header
/// (each `num_*` field is 4 bits wide).
pub const HIPC_MAX_DESCRIPTORS: usize = 15;

/// Maximum receive-list entries. Per-pointer mode encodes `2 + n` in the
/// 4-bit `recv_static_mode` field, capping `n` at 13.
pub const HIPC_MAX_RECV_LIST: usize = 13;

/// Protocol-specific writer for the data-words region of an HIPC request.
///
/// CMIF and TIPC implement this trait. [`HipcRequestBuilder::payload`] invokes
/// [`encode`](Self::encode) once it has computed the layout, written the HIPC
/// header and descriptor slots, and carved the data-words region. The writer
/// fills those bytes and bundles the surrounding [`Request`] (with its
/// already-populated descriptor slices) into a protocol-shaped output such as
/// `CmifRequest<'a>` or `TipcRequest<'a>`.
pub trait HipcPayload {
    /// Protocol-shaped value returned to the caller.
    type Output<'a>;
    /// Error this writer can report. Use [`core::convert::Infallible`] for
    /// writers that cannot fail.
    type Error;

    /// Bytes the writer needs inside the data-words region.
    ///
    /// Must be deterministic — `payload()` calls it once before computing
    /// layout and not again.
    fn encoded_len(&self) -> usize;

    /// Writes the payload into `dst` (exactly `encoded_len()` bytes) and
    /// bundles `hipc` into a protocol-shaped output.
    ///
    /// `dst` is the data-words region of the request buffer, exposed as bytes
    /// for the writer; `hipc` carries the surrounding descriptor / handle /
    /// recv-list slices.
    fn encode<'a>(
        self,
        hipc: Request<'a>,
        dst: &'a mut [u8],
    ) -> Result<Self::Output<'a>, Self::Error>;
}

/// Error returned by [`HipcRequestBuilder::payload`].
#[derive(Debug, thiserror::Error)]
pub enum BuildError<E> {
    /// The accumulated descriptors plus encoded payload exceed the request
    /// buffer size.
    #[error("request layout requires {needed} bytes, IPC buffer holds {limit}")]
    TooLarge {
        /// Total bytes implied by descriptors + payload.
        needed: usize,
        /// Capacity of the request buffer.
        limit: usize,
    },
    /// The payload writer reported an error.
    #[error("payload encode failed")]
    Payload(#[source] E),
}

/// Receive-list (Type C) configuration for the HIPC header.
///
/// Lowered to the 4-bit `recv_static_mode` field on the build path. Variants
/// mirror the four kernel-defined cases — see [`RECV_LIST_WIRE_NONE`] in the
/// wire module for the authoritative wire-encoding table.
///
/// The default is [`RecvListMode::None`].
#[derive(Debug, Default)]
pub(crate) enum RecvListMode {
    /// No recv-list; the server may not return Type-X pointer data (wire mode
    /// `0`).
    #[default]
    None,
    /// Server places returned pointer data inside the client's TLS message
    /// buffer, after the data words (wire mode `1`). No wire slot is reserved.
    ToMessageBuffer,
    /// One wire slot the server may subdivide for all returned pointer data
    /// (wire mode `2`). libnx fills the slot on receipt with the session's
    /// pointer buffer.
    SingleBuffer,
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
            RecvListMode::None | RecvListMode::ToMessageBuffer => 0,
            RecvListMode::SingleBuffer => 1,
            RecvListMode::Entries(v) => v.len(),
        }
    }

    /// Encodes the 4-bit `recv_static_mode` wire field.
    #[inline]
    pub fn to_raw(&self) -> u8 {
        match self {
            RecvListMode::None => RECV_LIST_WIRE_NONE,
            RecvListMode::ToMessageBuffer => RECV_LIST_WIRE_TO_MESSAGE_BUFFER,
            RecvListMode::SingleBuffer => RECV_LIST_WIRE_SINGLE_BUFFER,
            RecvListMode::Entries(v) => RECV_LIST_WIRE_SINGLE_BUFFER + v.len() as u8,
        }
    }
}

/// Mutable views into a request buffer being constructed.
///
/// Returned to a [`HipcPayload`] writer alongside the data-words `dst`. The
/// descriptor slices have already been populated by the builder; the writer
/// is free to bundle this struct verbatim into its protocol-shaped output.
#[derive(Debug)]
pub struct Request<'a> {
    /// Send static descriptors.
    pub send_statics: &'a mut [StaticDescriptor],
    /// Send buffer descriptors.
    pub send_buffers: &'a mut [BufferDescriptor],
    /// Receive buffer descriptors.
    pub recv_buffers: &'a mut [BufferDescriptor],
    /// Exchange buffer descriptors.
    pub exch_buffers: &'a mut [BufferDescriptor],
    /// Receive list entries.
    pub recv_list: &'a mut [RecvListEntry],
    /// Copy handle slots.
    pub copy_handles: &'a mut [RawHandle],
    /// Move handle slots.
    pub move_handles: &'a mut [RawHandle],
}

/// Builds an HIPC request.
///
/// Accumulates HIPC-level descriptors via fluent `with_*` methods and
/// finalizes via [`payload`](Self::payload), which writes into a
/// caller-supplied buffer. Storage is inline (`[T; HIPC_MAX_DESCRIPTORS]`); the
/// descriptor counts are bounded by the HIPC wire format. The builder itself
/// holds no buffer reference, so descriptor accumulation does not lock any
/// borrow on the destination buffer.
pub struct HipcRequestBuilder {
    message_type: MessageType,
    send_statics: ArrayVec<StaticDescriptor, HIPC_MAX_DESCRIPTORS>,
    send_buffers: ArrayVec<BufferDescriptor, HIPC_MAX_DESCRIPTORS>,
    recv_buffers: ArrayVec<BufferDescriptor, HIPC_MAX_DESCRIPTORS>,
    exch_buffers: ArrayVec<BufferDescriptor, HIPC_MAX_DESCRIPTORS>,
    copy_handles: ArrayVec<RawHandle, HIPC_MAX_DESCRIPTORS>,
    move_handles: ArrayVec<RawHandle, HIPC_MAX_DESCRIPTORS>,
    recv_list_mode: RecvListMode,
    send_pid: bool,
}

impl HipcRequestBuilder {
    /// Starts a new builder for the given message type.
    ///
    /// `message_type` accepts any value convertible into [`MessageType`] —
    /// typically a CMIF or TIPC `CommandType`. The destination buffer is
    /// supplied later, at [`payload`](Self::payload) time.
    #[inline]
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
        }
    }

    /// Replaces the message type chosen at construction.
    ///
    /// CMIF callers use this to switch between [`Request`] and
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

    /// Configures the receive-list to [`RecvListMode::ToMessageBuffer`] (wire
    /// mode `1`): the server places returned pointer data inside the client's
    /// TLS message buffer; no wire slot is reserved.
    ///
    /// # Panics
    ///
    /// Debug-panics if entries were already pushed via
    /// [`with_recv_list_entry`](Self::with_recv_list_entry); the two shapes are
    /// mutually exclusive on the wire.
    #[inline]
    pub fn with_recv_list_to_message_buffer(mut self) -> Self {
        debug_assert!(
            !matches!(self.recv_list_mode, RecvListMode::Entries(_)),
            "with_recv_list_to_message_buffer would discard pushed recv-list entries",
        );
        self.recv_list_mode = RecvListMode::ToMessageBuffer;
        self
    }

    /// Configures the receive-list to [`RecvListMode::SingleBuffer`] (wire
    /// mode `2`): one wire slot is reserved that the server may subdivide for
    /// all returned pointer data.
    ///
    /// # Panics
    ///
    /// Debug-panics if entries were already pushed via
    /// [`with_recv_list_entry`](Self::with_recv_list_entry); the two shapes are
    /// mutually exclusive on the wire.
    #[inline]
    pub fn with_recv_list_single_buffer(mut self) -> Self {
        debug_assert!(
            !matches!(self.recv_list_mode, RecvListMode::Entries(_)),
            "with_recv_list_single_buffer would discard pushed recv-list entries",
        );
        self.recv_list_mode = RecvListMode::SingleBuffer;
        self
    }

    /// Appends a per-pointer recv-list entry, transitioning the builder into
    /// [`RecvListMode::Entries`] (wire mode `2 + n`).
    ///
    /// # Panics
    ///
    /// Debug-panics if the current mode is [`RecvListMode::ToMessageBuffer`]
    /// or [`RecvListMode::SingleBuffer`]; those zero-entry shapes are mutually
    /// exclusive with the per-pointer shape on the wire.
    #[inline]
    pub fn with_recv_list_entry(mut self, entry: RecvListEntry) -> Self {
        match &mut self.recv_list_mode {
            RecvListMode::Entries(v) => v.push(entry),
            RecvListMode::None => {
                let mut v = ArrayVec::new();
                v.push(entry);
                self.recv_list_mode = RecvListMode::Entries(v);
            }
            RecvListMode::ToMessageBuffer | RecvListMode::SingleBuffer => {
                debug_assert!(
                    false,
                    "with_recv_list_entry called after a zero-entry recv-list mode was set",
                );
                let mut v = ArrayVec::new();
                v.push(entry);
                self.recv_list_mode = RecvListMode::Entries(v);
            }
        }
        self
    }

    /// Finalizes the request into `buf`. Computes the wire layout from the
    /// accumulated counts plus `payload.encoded_len()`, writes the HIPC header
    /// and descriptor slots via zerocopy, then invokes
    /// [`payload.encode`](HipcPayload::encode) on the carved data-words
    /// region.
    pub fn payload<'a, const N: usize, P: HipcPayload>(
        self,
        buf: &'a mut [u8; N],
        payload: P,
    ) -> Result<P::Output<'a>, BuildError<P::Error>> {
        let needed_payload = payload.encoded_len();
        let num_data_words = needed_payload.div_ceil(size_of::<u32>());

        let layout = Layout {
            send_statics: self.send_statics.len(),
            send_buffers: self.send_buffers.len(),
            recv_buffers: self.recv_buffers.len(),
            exch_buffers: self.exch_buffers.len(),
            num_data_words,
            recv_list_entries: self.recv_list_mode.wire_slot_count(),
            send_pid: self.send_pid,
            num_copy_handles: self.copy_handles.len(),
            num_move_handles: self.move_handles.len(),
        };

        let total_bytes = layout.total_bytes();
        if total_bytes > N {
            return Err(BuildError::TooLarge {
                needed: total_bytes,
                limit: N,
            });
        }

        let (hipc, data_bytes) = write_hipc(
            buf,
            self.message_type,
            &layout,
            &self.recv_list_mode,
            &self.send_statics,
            &self.send_buffers,
            &self.recv_buffers,
            &self.exch_buffers,
            &self.copy_handles,
            &self.move_handles,
        );

        let (dst, _padding) = data_bytes.split_at_mut(needed_payload);

        payload.encode(hipc, dst).map_err(BuildError::Payload)
    }
}

/// Wire-level layout of a finalized HIPC request.
struct Layout {
    send_statics: usize,
    send_buffers: usize,
    recv_buffers: usize,
    exch_buffers: usize,
    num_data_words: usize,
    recv_list_entries: usize,
    send_pid: bool,
    num_copy_handles: usize,
    num_move_handles: usize,
}

impl Layout {
    #[inline]
    fn has_special_header(&self) -> bool {
        self.send_pid || self.num_copy_handles > 0 || self.num_move_handles > 0
    }

    fn total_bytes(&self) -> usize {
        let mut total = size_of::<Header>();
        if self.has_special_header() {
            total += size_of::<SpecialHeader>();
            if self.send_pid {
                total += size_of::<u64>();
            }
        }
        total += self.num_copy_handles * size_of::<RawHandle>();
        total += self.num_move_handles * size_of::<RawHandle>();
        total += self.send_statics * size_of::<StaticDescriptor>();
        total += self.send_buffers * size_of::<BufferDescriptor>();
        total += self.recv_buffers * size_of::<BufferDescriptor>();
        total += self.exch_buffers * size_of::<BufferDescriptor>();
        total += self.num_data_words * size_of::<u32>();
        total += self.recv_list_entries * size_of::<RecvListEntry>();
        total
    }
}

/// Writes the HIPC header, special header, and descriptor slots into `buf`,
/// returning a [`Request`] with mutable views over each region.
///
/// The total size check has already been performed by the caller, so every
/// `mut_from_prefix*` call is infallible.
#[expect(clippy::too_many_arguments)]
fn write_hipc<'a, const N: usize>(
    buf: &'a mut [u8; N],
    message_type: MessageType,
    layout: &Layout,
    recv_list_mode: &RecvListMode,
    src_send_statics: &[StaticDescriptor],
    src_send_buffers: &[BufferDescriptor],
    src_recv_buffers: &[BufferDescriptor],
    src_exch_buffers: &[BufferDescriptor],
    src_copy_handles: &[RawHandle],
    src_move_handles: &[RawHandle],
) -> (Request<'a>, &'a mut [u8]) {
    let recv_static_mode = recv_list_mode.to_raw();
    let header = Header::new()
        .with_message_type(message_type.to_raw())
        .with_num_send_statics(layout.send_statics as u8)
        .with_num_send_buffers(layout.send_buffers as u8)
        .with_num_recv_buffers(layout.recv_buffers as u8)
        .with_num_exch_buffers(layout.exch_buffers as u8)
        .with_num_data_words(layout.num_data_words as u16)
        .with_recv_static_mode(recv_static_mode)
        .with_has_special_header(layout.has_special_header());

    let (hdr, buf) =
        Header::mut_from_prefix(&mut buf[..]).expect("internal: edge check guarantees buffer fits");
    *hdr = header;

    let buf = if layout.has_special_header() {
        let special = SpecialHeader::new()
            .with_send_pid(layout.send_pid)
            .with_num_copy_handles(layout.num_copy_handles as u8)
            .with_num_move_handles(layout.num_move_handles as u8);

        let (sp, buf) = SpecialHeader::mut_from_prefix(buf)
            .expect("internal: edge check guarantees buffer fits");
        *sp = special;

        if layout.send_pid {
            let (_, rest) = buf.split_at_mut(size_of::<u64>());
            rest
        } else {
            buf
        }
    } else {
        buf
    };

    let (copy_handles, buf) =
        <[RawHandle]>::mut_from_prefix_with_elems(buf, layout.num_copy_handles)
            .expect("internal: edge check guarantees buffer fits");
    copy_handles.copy_from_slice(src_copy_handles);

    let (move_handles, buf) =
        <[RawHandle]>::mut_from_prefix_with_elems(buf, layout.num_move_handles)
            .expect("internal: edge check guarantees buffer fits");
    move_handles.copy_from_slice(src_move_handles);

    let (send_statics, buf) =
        <[StaticDescriptor]>::mut_from_prefix_with_elems(buf, layout.send_statics)
            .expect("internal: edge check guarantees buffer fits");
    send_statics.copy_from_slice(src_send_statics);

    let (send_buffers, buf) =
        <[BufferDescriptor]>::mut_from_prefix_with_elems(buf, layout.send_buffers)
            .expect("internal: edge check guarantees buffer fits");
    send_buffers.copy_from_slice(src_send_buffers);

    let (recv_buffers, buf) =
        <[BufferDescriptor]>::mut_from_prefix_with_elems(buf, layout.recv_buffers)
            .expect("internal: edge check guarantees buffer fits");
    recv_buffers.copy_from_slice(src_recv_buffers);

    let (exch_buffers, buf) =
        <[BufferDescriptor]>::mut_from_prefix_with_elems(buf, layout.exch_buffers)
            .expect("internal: edge check guarantees buffer fits");
    exch_buffers.copy_from_slice(src_exch_buffers);

    let data_bytes_len = layout.num_data_words * size_of::<u32>();
    let (data_bytes, buf) = buf.split_at_mut(data_bytes_len);

    let (recv_list, _) =
        <[RecvListEntry]>::mut_from_prefix_with_elems(buf, layout.recv_list_entries)
            .expect("internal: edge check guarantees buffer fits");
    match recv_list_mode {
        RecvListMode::Entries(v) => recv_list.copy_from_slice(v),
        RecvListMode::SingleBuffer => recv_list[0] = RecvListEntry::default(),
        RecvListMode::None | RecvListMode::ToMessageBuffer => {
            // No wire slots reserved for these modes.
        }
    }

    let request = Request {
        send_statics,
        send_buffers,
        recv_buffers,
        exch_buffers,
        recv_list,
        copy_handles,
        move_handles,
    };
    (request, data_bytes)
}
