//! HIPC request building (client side).
//!
//! Only the build path is implemented in this crate — see the parent module's
//! "Server-side request parsing" section for why a `parse_request` counterpart
//! is intentionally absent.
//!
//! # DTO model
//!
//! [`HipcRequestBuilder`] accumulates HIPC-level descriptors (statics, buffers,
//! handles, recv-list) without holding any buffer reference and finalizes into
//! a [`HipcRequest`] DTO via [`HipcRequestBuilder::build`]. The DTO carries
//! every input needed to serialize the request — descriptors, handles,
//! recv-list configuration, send-PID flag, and the size of the data-words
//! region — but **no buffer reference**.
//!
//! Serialization happens via [`HipcRequest::write_to`], which writes the HIPC
//! header and all descriptor slots into a caller-supplied `&mut [u8; N]`
//! buffer and leaves the data-words region zero-initialized. Higher-level
//! protocols (CMIF, TIPC) wrap a [`HipcRequest`] in their own DTO and
//! delegate to [`write_to`](HipcRequest::write_to), then fill the data-words
//! region using [`data_words_offset`](HipcRequest::data_words_offset) to
//! locate it.
//!
//! Descriptor counts are bounded by the HIPC header's 4-bit fields, so the
//! DTO uses inline `[T; HIPC_MAX_DESCRIPTORS]` storage — no heap, no dynamic
//! allocation.

use core::{convert::Infallible, mem::size_of};

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

/// Error returned by [`HipcRequest::write_to`].
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
    /// A higher-level protocol writer reported an error.
    ///
    /// Reserved for protocol wrappers (CMIF / TIPC) that compose
    /// [`HipcRequest::write_to`] with their own data-words encoder; for the
    /// bare HIPC write the error parameter is [`Infallible`].
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
#[derive(Debug, Default, Clone)]
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

/// HIPC request DTO.
///
/// Self-contained value-type description of an HIPC request: message type,
/// every descriptor / handle that goes on the wire, recv-list configuration,
/// `send_pid` flag, and the size of the data-words region. Holds **no**
/// buffer reference. Constructed by [`HipcRequestBuilder::build`].
///
/// Serialize via [`write_to`](Self::write_to). The data-words region is left
/// zero-initialized; higher-level protocols (CMIF, TIPC) fill it themselves
/// using [`data_words_offset`](Self::data_words_offset) to locate the region.
#[derive(Debug, Clone)]
pub struct HipcRequest {
    message_type: MessageType,
    send_statics: ArrayVec<StaticDescriptor, HIPC_MAX_DESCRIPTORS>,
    send_buffers: ArrayVec<BufferDescriptor, HIPC_MAX_DESCRIPTORS>,
    recv_buffers: ArrayVec<BufferDescriptor, HIPC_MAX_DESCRIPTORS>,
    exch_buffers: ArrayVec<BufferDescriptor, HIPC_MAX_DESCRIPTORS>,
    copy_handles: ArrayVec<RawHandle, HIPC_MAX_DESCRIPTORS>,
    move_handles: ArrayVec<RawHandle, HIPC_MAX_DESCRIPTORS>,
    recv_list_mode: RecvListMode,
    send_pid: bool,
    data_words_size: usize,
}

impl HipcRequest {
    /// Returns the byte offset of the data-words region inside a serialized
    /// HIPC request.
    ///
    /// Higher-level protocols (CMIF, TIPC) call this after
    /// [`write_to`](Self::write_to) to locate where they should write their
    /// in-band payload.
    pub fn data_words_offset(&self) -> usize {
        self.layout().data_words_offset()
    }

    /// Returns the data-words region size in bytes (rounded up to a 4-byte
    /// boundary, matching the HIPC `num_data_words` wire field).
    pub fn data_words_size(&self) -> usize {
        self.layout().num_data_words * size_of::<u32>()
    }

    /// Writes the HIPC header and descriptor slots into `dst`.
    ///
    /// Leaves the data-words region zero-initialized. The total layout size
    /// must fit in `N`; otherwise [`BuildError::TooLarge`] is returned.
    pub fn write_to<const N: usize>(
        &self,
        dst: &mut [u8; N],
    ) -> Result<(), BuildError<Infallible>> {
        let layout = self.layout();
        let total_bytes = layout.total_bytes();
        if total_bytes > N {
            return Err(BuildError::TooLarge {
                needed: total_bytes,
                limit: N,
            });
        }

        write_hipc(
            dst,
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
        Ok(())
    }

    fn layout(&self) -> Layout {
        Layout {
            send_statics: self.send_statics.len(),
            send_buffers: self.send_buffers.len(),
            recv_buffers: self.recv_buffers.len(),
            exch_buffers: self.exch_buffers.len(),
            num_data_words: self.data_words_size.div_ceil(size_of::<u32>()),
            recv_list_entries: self.recv_list_mode.wire_slot_count(),
            send_pid: self.send_pid,
            num_copy_handles: self.copy_handles.len(),
            num_move_handles: self.move_handles.len(),
        }
    }
}

/// Fluent builder for an [`HipcRequest`].
///
/// Accumulates HIPC-level descriptors via `with_*` methods. Holds no buffer
/// reference. Finalize via [`build`](Self::build), which takes the
/// data-words region size and returns a self-contained [`HipcRequest`].
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
    /// not bound until [`HipcRequest::write_to`].
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

    /// Finalizes the builder into a [`HipcRequest`] DTO with the given
    /// data-words region size.
    pub fn build(self, data_words_size: usize) -> HipcRequest {
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
            data_words_size,
        }
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

    /// Byte offset of the data-words region from the start of the request.
    fn data_words_offset(&self) -> usize {
        let mut off = size_of::<Header>();
        if self.has_special_header() {
            off += size_of::<SpecialHeader>();
            if self.send_pid {
                off += size_of::<u64>();
            }
        }
        off += self.num_copy_handles * size_of::<RawHandle>();
        off += self.num_move_handles * size_of::<RawHandle>();
        off += self.send_statics * size_of::<StaticDescriptor>();
        off += self.send_buffers * size_of::<BufferDescriptor>();
        off += self.recv_buffers * size_of::<BufferDescriptor>();
        off += self.exch_buffers * size_of::<BufferDescriptor>();
        off
    }
}

/// Writes the HIPC header, special header, and descriptor slots into `buf`.
///
/// The total size check has already been performed by the caller, so every
/// `mut_from_prefix*` call is infallible. The data-words region is left
/// zero-initialized.
#[expect(clippy::too_many_arguments)]
fn write_hipc<const N: usize>(
    buf: &mut [u8; N],
    message_type: MessageType,
    layout: &Layout,
    recv_list_mode: &RecvListMode,
    src_send_statics: &[StaticDescriptor],
    src_send_buffers: &[BufferDescriptor],
    src_recv_buffers: &[BufferDescriptor],
    src_exch_buffers: &[BufferDescriptor],
    src_copy_handles: &[RawHandle],
    src_move_handles: &[RawHandle],
) {
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
    let (_data_bytes, buf) = buf.split_at_mut(data_bytes_len);

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
}
