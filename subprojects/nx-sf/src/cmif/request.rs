//! CMIF request building.
//!
//! This module contains request values and fluent builders for CMIF
//! requests. Serialization and response parsing live in sibling modules.
//!
//! # In-band payload model
//!
//! CMIF requests are layered on top of HIPC by attaching a typed in-band
//! payload to a [`HipcRequest`]: HIPC owns the envelope (descriptors,
//! handles, recv-list); the CMIF body owns everything in the data-words
//! region (the optional `DomainInHeader`, the `InHeader`, the raw rpc
//! payload, the input-object id tail, and the out-pointer-size table).
//!
//! Three body types implement [`HipcPayload`] for the three CMIF flavors:
//! [`CmifBody`] for plain and domain requests, [`CmifControlBody`] for
//! control requests, and [`CmifCloseBody`] for domain-object close.
//! Session-close has no in-band data and reuses the default `()` payload.

use core::{
    mem::{size_of, size_of_val},
    ptr,
};

use nx_svc::raw::Handle as RawHandle;

use super::{
    object_id::ObjectId,
    wire::{
        CMIF_HEADER_ALIGN, CMIF_MAX_OBJECTS, CommandType, DomainInHeader, DomainRequestType,
        InHeader,
    },
};
use crate::{
    array_vec::ArrayVec,
    hipc::{
        self, BufferDescriptor, BufferMode, HIPC_MAX_RECV_LIST, HipcPayload, HipcRequest,
        HipcRequestBuilder, RecvListEntry, StaticDescriptor,
    },
};

/// Layout error for CMIF request serialization.
///
/// CMIF body encoders cannot fail — HIPC reserves
/// `encoded_len.next_multiple_of(4)` bytes, so the destination slice is
/// always large enough by construction. Layout failures come from the
/// underlying HIPC request size check ([`hipc::WriteError`]).
pub type RequestLayoutError = hipc::WriteError;

/// Value-type description of a full CMIF request body.
///
/// Alias for a [`HipcRequest`] carrying a [`CmifBody`] payload. Inherits
/// [`HipcRequest::write_to`]. Most callers construct one through
/// [`CmifRequestBuilder`].
pub type CmifRequest<'a> = HipcRequest<CmifBody<'a>>;

/// Fluent builder for a full CMIF request.
///
/// Accumulates HIPC descriptors, handles, CMIF payload sizing, domain object
/// IDs, context tokens, and auto-buffer state. Finalizing produces a
/// self-contained [`CmifRequest`] that can serialize itself into the caller's
/// IPC buffer.
pub struct CmifRequestBuilder<'a> {
    hipc: HipcRequestBuilder,
    request_id: u32,
    context: u32,
    object_id: Option<ObjectId>,
    payload: &'a [u8],
    num_out_auto_buffers: u32,
    num_out_pointers: u32,
    objects: ArrayVec<ObjectId, CMIF_MAX_OBJECTS>,
    out_pointer_sizes: ArrayVec<u16, HIPC_MAX_RECV_LIST>,
    server_pointer_size: usize,
    cur_in_ptr_id: u8,
}

impl<'a> CmifRequestBuilder<'a> {
    /// Starts a new builder for the given command id.
    #[inline]
    pub fn new(request_id: u32) -> Self {
        Self {
            hipc: HipcRequestBuilder::new(CommandType::Request),
            request_id,
            context: 0,
            object_id: None,
            payload: &[],
            num_out_auto_buffers: 0,
            num_out_pointers: 0,
            objects: ArrayVec::new(),
            out_pointer_sizes: ArrayVec::new(),
            server_pointer_size: 0,
            cur_in_ptr_id: 0,
        }
    }

    /// Sets the server pointer-buffer capacity used by auto-buffer selection.
    ///
    /// Auto-buffers use inline pointer descriptors while enough pointer-buffer
    /// capacity remains, then fall back to mapped buffer descriptors.
    #[inline]
    pub fn with_pointer_buffer_size(mut self, size: usize) -> Self {
        self.server_pointer_size = size;
        self
    }

    /// Sets the context token for versioned requests.
    ///
    /// Non-zero values switch the HIPC message type to
    /// [`CommandType::RequestWithContext`] and set the CMIF header version to
    /// 1.
    #[inline]
    pub fn with_context(mut self, ctx: u32) -> Self {
        self.context = ctx;
        self
    }

    /// Sets the request payload bytes to copy into the CMIF data area.
    #[inline]
    pub fn with_data(mut self, data: &'a [u8]) -> Self {
        self.payload = data;
        self
    }

    /// Sets the request payload from a typed value via its zero-copy byte view.
    #[inline]
    pub fn with_data_value<T>(self, value: &'a T) -> Self
    where
        T: zerocopy::IntoBytes + zerocopy::Immutable,
    {
        self.with_data(value.as_bytes())
    }

    /// Marks the request as targeting an object inside a CMIF domain.
    #[inline]
    pub fn with_object_id(mut self, id: ObjectId) -> Self {
        self.object_id = Some(id);
        self
    }

    /// Includes the current process ID in the underlying HIPC request.
    #[inline]
    pub fn with_send_pid(mut self) -> Self {
        self.hipc = self.hipc.with_send_pid();
        self
    }

    /// Adds a mapped input buffer using a Type-A HIPC descriptor.
    #[inline]
    pub fn add_input_buffer_raw(
        mut self,
        buffer: *const u8,
        size: usize,
        mode: BufferMode,
    ) -> Self {
        self.hipc = self
            .hipc
            .with_send_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
        self
    }

    /// Adds a mapped output buffer using a Type-B HIPC descriptor.
    #[inline]
    pub fn add_output_buffer_raw(mut self, buffer: *mut u8, size: usize, mode: BufferMode) -> Self {
        self.hipc = self
            .hipc
            .with_recv_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
        self
    }

    /// Adds a mapped bidirectional buffer using a Type-W HIPC descriptor.
    #[inline]
    pub fn add_inout_buffer_raw(mut self, buffer: *mut u8, size: usize, mode: BufferMode) -> Self {
        self.hipc = self
            .hipc
            .with_exch_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
        self
    }

    /// Adds a mapped input buffer from a byte slice.
    ///
    /// Empty slices encode a null descriptor instead of taking a dangling
    /// pointer from the slice.
    #[inline]
    pub fn add_input_buffer(self, buffer: &[u8], mode: BufferMode) -> Self {
        let ptr = if buffer.is_empty() {
            ptr::null()
        } else {
            buffer.as_ptr()
        };
        self.add_input_buffer_raw(ptr, buffer.len(), mode)
    }

    /// Adds a mapped output buffer from a mutable byte slice.
    ///
    /// Empty slices encode a null descriptor instead of taking a dangling
    /// pointer from the slice.
    #[inline]
    pub fn add_output_buffer(self, buffer: &mut [u8], mode: BufferMode) -> Self {
        let ptr = if buffer.is_empty() {
            ptr::null_mut()
        } else {
            buffer.as_mut_ptr()
        };
        self.add_output_buffer_raw(ptr, buffer.len(), mode)
    }

    /// Adds an input pointer using a Type-X send-static descriptor.
    ///
    /// The pointer consumes one server pointer-buffer slot and advances the
    /// CMIF input pointer index.
    #[inline]
    pub fn add_in_pointer(mut self, buffer: *const u8, size: usize) -> Self {
        let id = self.cur_in_ptr_id;
        self.hipc = self
            .hipc
            .with_send_static(StaticDescriptor::new_send(buffer, size, id));
        self.cur_in_ptr_id += 1;
        self.server_pointer_size = self.server_pointer_size.saturating_sub(size);
        self
    }

    /// Adds a fixed-size output pointer using a Type-C recv-list entry.
    ///
    /// Fixed pointers do not add an out-pointer-size table entry.
    #[inline]
    pub fn add_out_fixed_pointer(mut self, buffer: *mut u8, size: usize) -> Self {
        self.hipc = self
            .hipc
            .with_recv_list_entry(RecvListEntry::new_recv(buffer, size));
        self.server_pointer_size = self.server_pointer_size.saturating_sub(size);
        self
    }

    /// Adds a variable-size output pointer using a Type-C recv-list entry.
    ///
    /// The pointer size is recorded in the CMIF out-pointer-size table.
    #[inline]
    pub fn add_out_pointer(mut self, buffer: *mut u8, size: usize) -> Self {
        self.hipc = self
            .hipc
            .with_recv_list_entry(RecvListEntry::new_recv(buffer, size));
        self.out_pointer_sizes.push(size as u16);
        self.num_out_pointers += 1;
        self.server_pointer_size = self.server_pointer_size.saturating_sub(size);
        self
    }

    /// Adds an input auto-buffer.
    ///
    /// The buffer is encoded as an inline pointer when it fits in the server
    /// pointer buffer, otherwise as a mapped buffer. The paired unused HIPC
    /// descriptor is still reserved with a zero descriptor to match CMIF
    /// layout rules.
    #[inline]
    pub fn add_in_auto_buffer(self, buffer: *const u8, size: usize, mode: BufferMode) -> Self {
        let mut s = self;
        if s.server_pointer_size > 0 && size <= s.server_pointer_size {
            let id = s.cur_in_ptr_id;
            s.hipc = s
                .hipc
                .with_send_static(StaticDescriptor::new_send(buffer, size, id));
            s.cur_in_ptr_id += 1;
            s.hipc = s
                .hipc
                .with_send_buffer(BufferDescriptor::new_buffer(ptr::null(), 0, mode));
            s.server_pointer_size = s.server_pointer_size.saturating_sub(size);
        } else {
            let id = s.cur_in_ptr_id;
            s.hipc = s
                .hipc
                .with_send_static(StaticDescriptor::new_send(ptr::null(), 0, id));
            s.cur_in_ptr_id += 1;
            s.hipc = s
                .hipc
                .with_send_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
        }
        s
    }

    /// Adds an output auto-buffer.
    ///
    /// The buffer is encoded as an inline output pointer when it fits in the
    /// server pointer buffer, otherwise as a mapped output buffer. The OPT
    /// entry records the inline size or zero for the mapped path.
    #[inline]
    pub fn add_out_auto_buffer(self, buffer: *mut u8, size: usize, mode: BufferMode) -> Self {
        let mut s = self;
        s.num_out_auto_buffers += 1;
        if s.server_pointer_size > 0 && size <= s.server_pointer_size {
            s.hipc = s
                .hipc
                .with_recv_list_entry(RecvListEntry::new_recv(buffer, size));
            s.hipc = s
                .hipc
                .with_recv_buffer(BufferDescriptor::new_buffer(ptr::null(), 0, mode));
            s.out_pointer_sizes.push(size as u16);
            s.server_pointer_size = s.server_pointer_size.saturating_sub(size);
        } else {
            s.hipc = s
                .hipc
                .with_recv_list_entry(RecvListEntry::new_recv(ptr::null_mut(), 0));
            s.hipc = s
                .hipc
                .with_recv_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
            s.out_pointer_sizes.push(0);
        }
        s
    }

    /// Adds a domain input object ID to pass with the request.
    #[inline]
    pub fn add_object(mut self, id: ObjectId) -> Self {
        self.objects.push(id);
        self
    }

    /// Adds a copy handle to pass with the underlying HIPC request.
    #[inline]
    pub fn add_copy_handle(mut self, handle: RawHandle) -> Self {
        self.hipc = self.hipc.with_copy_handle(handle);
        self
    }

    /// Adds a move handle to transfer with the underlying HIPC request.
    #[inline]
    pub fn add_move_handle(mut self, handle: RawHandle) -> Self {
        self.hipc = self.hipc.with_move_handle(handle);
        self
    }

    /// Finalizes the request value.
    pub fn build(self) -> CmifRequest<'a> {
        let message_type = if self.context != 0 {
            CommandType::RequestWithContext
        } else {
            CommandType::Request
        };
        let body = CmifBody {
            request_id: self.request_id,
            context: self.context,
            object_id: self.object_id,
            payload: self.payload,
            objects: self.objects,
            out_pointer_sizes: self.out_pointer_sizes,
            num_out_auto_buffers: self.num_out_auto_buffers,
            num_out_pointers: self.num_out_pointers,
        };
        self.hipc
            .set_message_type(message_type)
            .with_payload(body)
            .build()
    }
}

/// CMIF control request body.
///
/// Alias for a [`HipcRequest`] carrying a [`CmifControlBody`] payload.
/// Control requests are used for operations such as domain conversion,
/// cloning, and pointer-buffer-size queries.
pub type CmifControlRequest<'a> = HipcRequest<CmifControlBody<'a>>;

/// Fluent builder for a CMIF control request.
///
/// Accumulates a control command ID and optional payload before producing a
/// control request or writing it directly into an IPC buffer.
pub struct CmifControlRequestBuilder<'a> {
    hipc: HipcRequestBuilder,
    request_id: u32,
    payload: &'a [u8],
}

impl<'a> CmifControlRequestBuilder<'a> {
    /// Starts a new control-request builder.
    #[inline]
    pub fn new(request_id: u32) -> Self {
        Self {
            hipc: HipcRequestBuilder::new(CommandType::Control),
            request_id,
            payload: &[],
        }
    }

    /// Sets the request payload data.
    #[inline]
    pub fn data(mut self, data: &'a [u8]) -> Self {
        self.payload = data;
        self
    }

    /// Finalizes the request value.
    pub fn build(self) -> CmifControlRequest<'a> {
        let body = CmifControlBody {
            request_id: self.request_id,
            payload: self.payload,
        };
        self.hipc.with_payload(body).build()
    }
}

/// CMIF close request body.
///
/// Represents either a plain session close (no in-band data) or a
/// domain-object close (carries a [`CmifCloseBody`] with the target object
/// id).
#[derive(Debug, Clone)]
pub enum CmifCloseRequest {
    /// Session close.
    Session(HipcRequest),
    /// Domain object close.
    DomainObject(HipcRequest<CmifCloseBody>),
}

impl CmifCloseRequest {
    /// Creates a session-close request.
    pub fn session() -> Self {
        Self::Session(HipcRequestBuilder::new(CommandType::Close).build())
    }

    /// Creates a domain-object close request.
    pub fn domain_object(object_id: ObjectId) -> Self {
        let body = CmifCloseBody { object_id };
        Self::DomainObject(
            HipcRequestBuilder::new(CommandType::Request)
                .with_payload(body)
                .build(),
        )
    }

    /// Writes the close request into `dst`.
    pub fn write_to<const N: usize>(&self, dst: &mut [u8; N]) -> Result<(), RequestLayoutError> {
        match self {
            Self::Session(hipc) => hipc.write_to(dst),
            Self::DomainObject(hipc) => hipc.write_to(dst),
        }
    }
}

/// In-band body for a CMIF request or domain request.
///
/// Encodes (optionally) the `DomainInHeader`, the `InHeader`, the raw rpc
/// payload, the input-object id tail, and the out-pointer-size table at the
/// region tail.
#[derive(Debug, Clone)]
pub struct CmifBody<'a> {
    request_id: u32,
    context: u32,
    object_id: Option<ObjectId>,
    payload: &'a [u8],
    objects: ArrayVec<ObjectId, CMIF_MAX_OBJECTS>,
    out_pointer_sizes: ArrayVec<u16, HIPC_MAX_RECV_LIST>,
    num_out_auto_buffers: u32,
    num_out_pointers: u32,
}

impl CmifBody<'_> {
    fn opt_size(&self) -> usize {
        size_of::<u16>() * (self.num_out_auto_buffers + self.num_out_pointers) as usize
    }

    fn cmif_version(&self) -> u32 {
        if self.context != 0 { 1 } else { 0 }
    }

    fn in_header_token(&self) -> u32 {
        if self.object_id.is_some() {
            0
        } else {
            self.context
        }
    }
}

impl HipcPayload for CmifBody<'_> {
    /// Sums the in-band sections this body emits: alignment slack for the
    /// leading pad ([`CMIF_HEADER_ALIGN`]), the optional `DomainInHeader`
    /// plus its trailing input-object id table (domain requests only), the
    /// always-present `InHeader`, the raw rpc payload, a half-word align-up
    /// before the trailing tables, and the out-pointer-size table.
    fn encoded_len(&self) -> usize {
        let mut n: usize = CMIF_HEADER_ALIGN;
        if self.object_id.is_some() {
            n += size_of::<DomainInHeader>() + self.objects.len() * size_of::<u32>();
        }
        n += size_of::<InHeader>() + self.payload.len();
        n = (n + 1) & !1;
        n += self.opt_size();
        n
    }

    /// Writes the CMIF in-band body for a plain or domain request.
    ///
    /// Splits the out-pointer-size table off the region tail, skips the
    /// [`CMIF_HEADER_ALIGN`] alignment pad at the head, then writes — in
    /// order — the optional `DomainInHeader`, the `InHeader`, the raw rpc
    /// payload, and the trailing input-object id table for domain requests.
    fn write_to(&self, dst: &mut [u8]) {
        let body_len = self.encoded_len();
        let cmif_region = &mut dst[..body_len];

        // The out-pointer-size table lives at the region tail; split it off
        // first so the head can be written sequentially.
        let opt_len = self.opt_size();
        let split = cmif_region.len() - opt_len;
        let (cmif_region, opt_bytes) = cmif_region.split_at_mut(split);
        write_section(opt_bytes, self.out_pointer_sizes.as_slice());

        // Skip alignment padding before the CMIF in-band headers.
        let pad = cmif_region.as_ptr().align_offset(CMIF_HEADER_ALIGN);
        let (_padding, buf) = cmif_region.split_at_mut(pad);

        // Optional DomainInHeader for domain requests.
        let buf = if let Some(object_id) = self.object_id {
            let payload_size = size_of::<InHeader>() as u16 + self.payload.len() as u16;
            write_cmif_domain_in_header(
                buf,
                DomainRequestType::SendMessage,
                self.objects.len(),
                payload_size,
                object_id,
                self.context,
            )
        } else {
            buf
        };

        // InHeader (always present).
        let buf = write_cmif_in_header(
            buf,
            self.request_id,
            self.cmif_version(),
            self.in_header_token(),
        );

        // Raw rpc payload.
        let buf = write_section(buf, self.payload);

        // Input-object IDs (domain requests only).
        if self.object_id.is_some() {
            write_section(buf, self.objects.as_slice());
        }
    }
}

/// In-band body for a CMIF control request.
///
/// Encodes the `InHeader` and the control-command payload, with no domain
/// framing and no out-pointer-size table.
#[derive(Debug, Clone)]
pub struct CmifControlBody<'a> {
    request_id: u32,
    payload: &'a [u8],
}

impl HipcPayload for CmifControlBody<'_> {
    /// Alignment slack for the leading pad ([`CMIF_HEADER_ALIGN`]), plus the
    /// `InHeader` and the control-command payload. No domain framing and no
    /// out-pointer table.
    fn encoded_len(&self) -> usize {
        CMIF_HEADER_ALIGN + size_of::<InHeader>() + self.payload.len()
    }

    /// Writes a CMIF control body: skip the [`CMIF_HEADER_ALIGN`] alignment
    /// pad, then the `InHeader` (with `version = 0`, `token = 0`) followed by
    /// the raw control-command payload.
    fn write_to(&self, dst: &mut [u8]) {
        let body_len = self.encoded_len();
        let region = &mut dst[..body_len];

        // Skip alignment padding before the CMIF in-band headers.
        let pad = region.as_ptr().align_offset(CMIF_HEADER_ALIGN);
        let (_padding, buf) = region.split_at_mut(pad);

        // InHeader (always present).
        let buf = write_cmif_in_header(buf, self.request_id, 0, 0);

        // Raw rpc payload.
        write_section(buf, self.payload);
    }
}

/// In-band body for a CMIF domain-object close request.
///
/// Encodes a `DomainInHeader` with `request_type = Close` and an empty
/// payload. Plain session-close carries no in-band data and uses the
/// default `()` payload instead.
#[derive(Debug, Clone)]
pub struct CmifCloseBody {
    object_id: ObjectId,
}

impl HipcPayload for CmifCloseBody {
    /// A `DomainInHeader` plus alignment slack for the leading pad
    /// ([`CMIF_HEADER_ALIGN`]). No `InHeader`, no payload, and no
    /// input-object id tail.
    fn encoded_len(&self) -> usize {
        size_of::<DomainInHeader>() + CMIF_HEADER_ALIGN
    }

    /// Writes a CMIF domain-object close body: skip the [`CMIF_HEADER_ALIGN`]
    /// alignment pad, then a single `DomainInHeader` with `request_type =
    /// Close` targeting the held object ID.
    fn write_to(&self, dst: &mut [u8]) {
        let region = &mut dst[..self.encoded_len()];

        // Skip alignment padding before the CMIF in-band headers.
        let pad = region.as_ptr().align_offset(CMIF_HEADER_ALIGN);
        let (_padding, buf) = region.split_at_mut(pad);

        // DomainInHeader carrying the `Close` request type; no InHeader,
        // no payload, no input-object id tail.
        write_cmif_domain_in_header(buf, DomainRequestType::Close, 0, 0, self.object_id, 0);
    }
}

/// Writes a CMIF [`InHeader`] into `buf` and returns the remaining tail.
///
/// Centralizes the magic constant so each body only supplies the fields it
/// chooses (`version`, `token`). Mirrors the `write_header` helper in
/// [`hipc::request`].
#[inline]
fn write_cmif_in_header(buf: &mut [u8], command_id: u32, version: u32, token: u32) -> &mut [u8] {
    let header = InHeader {
        magic: super::wire::IN_HEADER_MAGIC,
        version,
        command_id,
        token,
    };
    write_section(buf, &header)
}

/// Writes a CMIF [`DomainInHeader`] into `buf` and returns the remaining tail.
///
/// Centralizes the field layout so each caller only supplies the fields it
/// chooses (`request_type`, `num_in_objects`, `data_size`, `object_id`,
/// `token`). Mirrors [`write_cmif_in_header`] for the domain framing header.
#[inline]
fn write_cmif_domain_in_header(
    buf: &mut [u8],
    request_type: DomainRequestType,
    num_in_objects: usize,
    data_size: u16,
    object_id: ObjectId,
    token: u32,
) -> &mut [u8] {
    let header = DomainInHeader {
        request_type: request_type as u8,
        num_in_objects: num_in_objects as u8,
        data_size,
        object_id: object_id.to_raw(),
        _padding: 0,
        token,
    };
    write_section(buf, &header)
}

/// Writes `value`'s bytes into the prefix of `buf` and returns the tail.
///
/// Mirrors the helper used by [`hipc::request`] so CMIF bodies thread the
/// destination buffer through a chain of typed sections instead of repeated
/// `mut_from_prefix` / `copy_from_slice` calls. [`CmifBody::encoded_len`]
/// guarantees the prefix fits, so the inner `write_to` call is infallible.
#[inline]
fn write_section<'a, T>(buf: &'a mut [u8], value: &T) -> &'a mut [u8]
where
    T: zerocopy::IntoBytes + zerocopy::Immutable + ?Sized,
{
    let (buf, tail) = buf.split_at_mut(size_of_val(value));
    value
        .write_to(buf)
        .expect("internal: encoded_len guarantees fit");
    tail
}
