//! CMIF (Command Message Interface Format) protocol implementation.
//!
//! CMIF is the command serialization layer built on top of HIPC. It provides
//! structured message formatting with magic headers for validation, command
//! IDs for method dispatch, and domain support for object multiplexing.
//!
//! # Protocol Stack
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │  Service APIs (fs, sm, hid, etc.)   │  Application layer
//! ├─────────────────────────────────────┤
//! │  CMIF  ← this module                │  Command serialization
//! ├─────────────────────────────────────┤
//! │  HIPC                               │  Message framing & descriptors
//! ├─────────────────────────────────────┤
//! │  Kernel SVCs (SendSyncRequest, etc) │  Transport
//! └─────────────────────────────────────┘
//! ```
//!
//! # Message Format
//!
//! CMIF messages are embedded within the HIPC data words section:
//!
//! **Non-Domain Request:**
//! ```text
//! [HIPC Header + Descriptors]
//! [Padding to 16-byte alignment]
//! [CmifInHeader (16 bytes): magic="SFCI", version, command_id, token]
//! [Payload data]
//! [Output pointer size table]
//! ```
//!
//! **Domain Request:**
//! ```text
//! [HIPC Header + Descriptors]
//! [Padding to 16-byte alignment]
//! [DomainInHeader (16 bytes): type, object_id, token]
//! [CmifInHeader (16 bytes)]
//! [Payload data]
//! [Object IDs array]
//! [Output pointer size table]
//! ```
//!
//! # Magic Numbers
//!
//! - `"SFCI"` (0x49434653): Service Framework Command Input
//! - `"SFCO"` (0x4F434653): Service Framework Command Output
//!
//! # Domains
//!
//! Domains allow multiplexing multiple service objects over a single session
//! handle, reducing kernel resource usage. Each object within a domain is
//! identified by a 32-bit [`ObjectId`].
//!
//! # Builder model
//!
//! [`CmifRequestBuilder`] is the high-level entry point for full CMIF
//! requests. It wraps a [`hipc::HipcRequestBuilder`], absorbs both HIPC
//! descriptor management and CMIF in-band state, and finalizes via
//! [`send`](CmifRequestBuilder::send), which takes the destination buffer
//! and returns the carved-out payload data slice ready to be filled.
//! The builder itself holds no buffer reference, so descriptor accumulation
//! does not lock any borrow on the destination buffer.
//!
//! The underlying value-type description is [`CmifRequest`]: a plain struct
//! with no lifetime that describes the request body and knows how to write
//! itself into a byte buffer via [`CmifRequest::write_to`]. It also
//! implements [`hipc::HipcPayload`] so it can be handed to a
//! [`hipc::HipcRequestBuilder`] directly when callers want custom descriptor
//! management. For control requests use [`CmifControlPayload`]; for close
//! requests use [`CmifClosePayload`]. Both also implement
//! [`hipc::HipcPayload`].
//!
//! # References
//!
//! - [Switchbrew IPC Marshalling](https://switchbrew.org/wiki/IPC_Marshalling)
//! - libnx `sf/cmif.h` (fincs, SciresM)

use core::{convert::Infallible, mem::size_of, ptr};

use nx_svc::raw::Handle as RawHandle;
use nx_sys_thread_tls::IPC_BUFFER_SIZE;
use static_assertions::const_assert_eq;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::hipc::{
    self, ArrayVec, BufferDescriptor, BufferMode, HIPC_MAX_RECV_LIST, HipcRequest,
    HipcRequestBuilder, RecvListEntry, StaticDescriptor,
};

/// Magic number for CMIF input headers ("SFCI" - Service Framework Command Input).
const IN_HEADER_MAGIC: u32 = 0x49434653;

/// Magic number for CMIF output headers ("SFCO" - Service Framework Command Output).
const OUT_HEADER_MAGIC: u32 = 0x4F434653;

/// Maximum number of domain objects passed in a single CMIF request.
pub const CMIF_MAX_OBJECTS: usize = 8;

/// Layout error for CMIF requests.
///
/// Alias for [`hipc::BuildError`] over [`Infallible`] since the CMIF payload
/// writers cannot fail — the only failure mode is a layout overflow surfaced
/// by the underlying HIPC builder.
pub type RequestLayoutError = hipc::BuildError<Infallible>;

/// Value-type description of a full CMIF request body.
///
/// Holds no buffer reference: it describes the optional [`DomainInHeader`],
/// the [`InHeader`], the payload data area size, the optional domain objects
/// array, and the out-pointer-size table (OPT). Encodes itself into a
/// destination byte buffer via [`write_to`](Self::write_to), returning the
/// carved-out payload data slice for the caller to fill.
///
/// Most callers construct this indirectly via [`CmifRequestBuilder`], which
/// absorbs HIPC descriptor management. Use `CmifRequest` directly when
/// driving a [`HipcRequestBuilder`] with custom descriptors — it also
/// implements [`HipcPayload`].
///
/// # Why not `zerocopy::IntoBytes`?
///
/// A CMIF request has a variable on-wire layout: the optional
/// `DomainInHeader`, the domain objects array, the payload data area and the
/// OPT all have caller-determined sizes. That violates the fixed-layout
/// invariants `zerocopy::IntoBytes` requires. The fixed sub-headers
/// ([`InHeader`], [`DomainInHeader`]) implement `IntoBytes` and remain
/// available for callers that want raw byte access to those parts.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CmifRequest<'a> {
    hipc: HipcRequest,
    request_id: u32,
    context: u32,
    object_id: Option<ObjectId>,
    data: RequestData<'a>,
    num_in_auto_buffers: u32,
    num_out_auto_buffers: u32,
    num_in_pointers: u32,
    num_out_pointers: u32,
    num_out_fixed_pointers: u32,
    objects: ArrayVec<u32, CMIF_MAX_OBJECTS>,
    out_pointer_sizes: ArrayVec<u16, HIPC_MAX_RECV_LIST>,
}

impl<'a> CmifRequest<'a> {
    fn data_len(&self) -> usize {
        match self.data {
            RequestData::Borrowed(data) => data.len(),
            RequestData::Sized(size) => size,
        }
    }

    fn data_bytes(&self) -> &[u8] {
        match self.data {
            RequestData::Borrowed(data) => data,
            RequestData::Sized(_) => &[],
        }
    }

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

    fn encoded_len(&self) -> usize {
        let mut n: usize = 16;
        if self.object_id.is_some() {
            n += size_of::<DomainInHeader>() + self.objects.len() * size_of::<u32>();
        }
        n += size_of::<InHeader>() + self.data_len();
        n = (n + 1) & !1;
        n += self.opt_size();
        n
    }

    /// Writes the CMIF request into `dst`.
    pub fn write_to<const N: usize>(&self, dst: &mut [u8; N]) -> Result<(), RequestLayoutError> {
        self.hipc.write_to(dst)?;

        let body_len = self.encoded_len();
        let data_words = self.hipc.data_words_size();
        if body_len > data_words {
            return Err(hipc::BuildError::TooLarge {
                needed: body_len,
                limit: data_words,
            });
        }

        let start = self.hipc.data_words_offset();
        let cmif_region = &mut dst[start..start + body_len];

        let opt_len = self.opt_size();
        let split = cmif_region.len() - opt_len;
        let (cmif_region, opt_bytes) = cmif_region.split_at_mut(split);
        let (out_pointer_sizes, _) =
            <[u16]>::mut_from_prefix_with_elems(opt_bytes, opt_len / size_of::<u16>())
                .expect("internal: encoded_len guarantees fit");
        out_pointer_sizes.copy_from_slice(self.out_pointer_sizes.as_slice());

        let pad = cmif_region.as_ptr().align_offset(16);
        let (_padding, aligned) = cmif_region.split_at_mut(pad);

        let aligned = if let Some(object_id) = self.object_id {
            let payload_size = size_of::<InHeader>() as u16 + self.data_len() as u16;
            let (dom_hdr, rest) = DomainInHeader::mut_from_prefix(aligned)
                .expect("internal: encoded_len guarantees fit");
            *dom_hdr = DomainInHeader {
                request_type: DomainRequestType::SendMessage as u8,
                num_in_objects: self.objects.len() as u8,
                data_size: payload_size,
                object_id: object_id.to_raw(),
                _padding: 0,
                token: self.context,
            };
            rest
        } else {
            aligned
        };

        let (in_hdr, rest) =
            InHeader::mut_from_prefix(aligned).expect("internal: encoded_len guarantees fit");
        *in_hdr = InHeader {
            magic: IN_HEADER_MAGIC,
            version: self.cmif_version(),
            command_id: self.request_id,
            token: self.in_header_token(),
        };

        let (data, rest) = rest.split_at_mut(self.data_len());
        data.copy_from_slice(self.data_bytes());

        if self.object_id.is_some() {
            let (objects, _) = <[u32]>::mut_from_prefix_with_elems(rest, self.objects.len())
                .expect("internal: encoded_len guarantees fit");
            objects.copy_from_slice(self.objects.as_slice());
        }

        Ok(())
    }
}

/// CMIF control request DTO.
#[derive(Debug, Clone)]
pub struct CmifControlRequest<'a> {
    hipc: HipcRequest,
    request_id: u32,
    data: RequestData<'a>,
}

impl<'a> CmifControlRequest<'a> {
    fn encoded_len(&self) -> usize {
        16 + size_of::<InHeader>() + self.data_len()
    }

    fn data_len(&self) -> usize {
        match self.data {
            RequestData::Borrowed(data) => data.len(),
            RequestData::Sized(size) => size,
        }
    }

    fn data_bytes(&self) -> &[u8] {
        match self.data {
            RequestData::Borrowed(data) => data,
            RequestData::Sized(_) => &[],
        }
    }

    /// Writes the control request into `dst`.
    pub fn write_to<const N: usize>(&self, dst: &mut [u8; N]) -> Result<(), RequestLayoutError> {
        self.hipc.write_to(dst)?;

        let body_len = self.encoded_len();
        let data_words = self.hipc.data_words_size();
        if body_len > data_words {
            return Err(hipc::BuildError::TooLarge {
                needed: body_len,
                limit: data_words,
            });
        }

        let start = self.hipc.data_words_offset();
        let region = &mut dst[start..start + body_len];
        let pad = region.as_ptr().align_offset(16);
        let (_padding, aligned) = region.split_at_mut(pad);

        let (hdr, rest) =
            InHeader::mut_from_prefix(aligned).expect("internal: encoded_len guarantees fit");
        *hdr = InHeader {
            magic: IN_HEADER_MAGIC,
            version: 0,
            command_id: self.request_id,
            token: 0,
        };

        let (payload, _) = rest.split_at_mut(self.data_len());
        payload.copy_from_slice(self.data_bytes());
        Ok(())
    }
}

/// Fluent builder for a CMIF control request.
pub struct CmifControlRequestBuilder<'a> {
    hipc: HipcRequestBuilder,
    request_id: u32,
    data: RequestData<'a>,
}

impl<'a> CmifControlRequestBuilder<'a> {
    fn data_len(&self) -> usize {
        match self.data {
            RequestData::Borrowed(data) => data.len(),
            RequestData::Sized(size) => size,
        }
    }

    /// Starts a new control-request builder.
    #[inline]
    pub fn new(request_id: u32) -> Self {
        Self {
            hipc: HipcRequestBuilder::new(CommandType::Control),
            request_id,
            data: RequestData::Borrowed(&[]),
        }
    }

    /// Sets the request payload data.
    #[inline]
    pub fn data(mut self, data: &'a [u8]) -> Self {
        self.data = RequestData::Borrowed(data);
        self
    }

    /// Reserves a payload of `size` bytes for later population.
    #[inline]
    pub fn data_size(mut self, size: usize) -> Self {
        self.data = RequestData::Sized(size);
        self
    }

    /// Finalizes the request DTO.
    pub fn build(self) -> CmifControlRequest<'a> {
        let encoded_len = self.encoded_len();
        let hipc = self.hipc.build(encoded_len);
        CmifControlRequest {
            hipc,
            request_id: self.request_id,
            data: self.data,
        }
    }

    /// Writes the request into `dst` and returns the payload slice.
    #[inline]
    pub fn send<const N: usize>(self, dst: &mut [u8; N]) -> Result<&mut [u8], RequestLayoutError> {
        let req = self.build();
        let payload_len = req.data_len();
        req.write_to(dst)?;
        let start = req.hipc.data_words_offset() + 16;
        Ok(&mut dst[start..start + payload_len])
    }

    fn encoded_len(&self) -> usize {
        16 + size_of::<InHeader>() + self.data_len()
    }
}

/// CMIF close request DTO.
#[derive(Debug, Clone)]
pub enum CmifCloseRequest {
    /// Session close.
    Session(HipcRequest),
    /// Domain object close.
    DomainObject {
        hipc: HipcRequest,
        object_id: ObjectId,
    },
}

impl CmifCloseRequest {
    /// Creates a session-close request.
    pub fn session() -> Self {
        Self::Session(HipcRequestBuilder::new(CommandType::Close).build(0))
    }

    /// Creates a domain-object close request.
    pub fn domain_object(object_id: ObjectId) -> Self {
        Self::DomainObject {
            hipc: HipcRequestBuilder::new(CommandType::Request)
                .build(size_of::<DomainInHeader>() + 16),
            object_id,
        }
    }

    /// Writes the close request into `dst`.
    pub fn write_to<const N: usize>(&self, dst: &mut [u8; N]) -> Result<(), RequestLayoutError> {
        match self {
            Self::Session(hipc) => hipc.write_to(dst),
            Self::DomainObject { hipc, object_id } => {
                hipc.write_to(dst)?;
                let start = hipc.data_words_offset();
                let region = &mut dst[start..start + size_of::<DomainInHeader>() + 16];
                let pad = region.as_ptr().align_offset(16);
                let (_padding, aligned) = region.split_at_mut(pad);

                let (dom_hdr, _) = DomainInHeader::mut_from_prefix(aligned)
                    .expect("internal: encoded_len guarantees fit");
                *dom_hdr = DomainInHeader {
                    request_type: DomainRequestType::Close as u8,
                    num_in_objects: 0,
                    data_size: 0,
                    object_id: object_id.to_raw(),
                    _padding: 0,
                    token: 0,
                };
                Ok(())
            }
        }
    }
}

/// Fluent builder for a full CMIF request.
pub struct CmifRequestBuilder<'a> {
    hipc: HipcRequestBuilder,
    request_id: u32,
    context: u32,
    object_id: Option<ObjectId>,
    data: RequestData<'a>,
    num_in_auto_buffers: u32,
    num_out_auto_buffers: u32,
    num_in_pointers: u32,
    num_out_pointers: u32,
    num_out_fixed_pointers: u32,
    objects: ArrayVec<u32, CMIF_MAX_OBJECTS>,
    out_pointer_sizes: ArrayVec<u16, HIPC_MAX_RECV_LIST>,
    server_pointer_size: usize,
    cur_in_ptr_id: u8,
}

#[derive(Debug, Clone, Copy)]
enum RequestData<'a> {
    Borrowed(&'a [u8]),
    Sized(usize),
}

impl<'a> CmifRequestBuilder<'a> {
    fn data_len(&self) -> usize {
        match self.data {
            RequestData::Borrowed(data) => data.len(),
            RequestData::Sized(size) => size,
        }
    }

    /// Starts a new builder for the given command id.
    #[inline]
    pub fn new(request_id: u32) -> Self {
        Self {
            hipc: HipcRequestBuilder::new(CommandType::Request),
            request_id,
            context: 0,
            object_id: None,
            data: RequestData::Borrowed(&[]),
            num_in_auto_buffers: 0,
            num_out_auto_buffers: 0,
            num_in_pointers: 0,
            num_out_pointers: 0,
            num_out_fixed_pointers: 0,
            objects: ArrayVec::new(),
            out_pointer_sizes: ArrayVec::new(),
            server_pointer_size: 0,
            cur_in_ptr_id: 0,
        }
    }

    /// Sets the server's pointer-buffer capacity (used by auto-buffer logic
    /// to decide between inline-pointer and mapped-buffer encoding).
    #[inline]
    pub fn pointer_buffer_size(mut self, size: usize) -> Self {
        self.server_pointer_size = size;
        self
    }

    /// Sets the context token for versioning. Non-zero values switch the HIPC
    /// message type to [`CommandType::RequestWithContext`] at send time and
    /// bump the [`InHeader`] version to 1.
    #[inline]
    pub fn context(mut self, ctx: u32) -> Self {
        self.context = ctx;
        self
    }

    /// Sets the request payload data.
    #[inline]
    pub fn data(mut self, data: &'a [u8]) -> Self {
        self.data = RequestData::Borrowed(data);
        self
    }

    /// Reserves a payload of `size` bytes for the caller to fill after send.
    #[inline]
    pub fn data_size(mut self, size: usize) -> Self {
        self.data = RequestData::Sized(size);
        self
    }

    /// Marks this request as targeting a domain object.
    #[inline]
    pub fn object_id(mut self, id: ObjectId) -> Self {
        self.object_id = Some(id);
        self
    }

    /// Enables sending the process ID alongside the request.
    #[inline]
    pub fn send_pid(mut self) -> Self {
        self.hipc = self.hipc.with_send_pid();
        self
    }

    /// Adds a mapped input buffer (Type A).
    #[inline]
    pub fn add_in_buffer(mut self, buffer: *const u8, size: usize, mode: BufferMode) -> Self {
        self.hipc = self
            .hipc
            .with_send_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
        self
    }

    /// Adds a mapped output buffer (Type B).
    #[inline]
    pub fn add_out_buffer(mut self, buffer: *mut u8, size: usize, mode: BufferMode) -> Self {
        self.hipc = self
            .hipc
            .with_recv_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
        self
    }

    /// Adds a mapped input buffer (Type A) from a byte slice.
    ///
    /// Slice-typed wrapper over [`add_in_buffer`](Self::add_in_buffer): the
    /// caller passes a borrowed buffer instead of a raw pointer and length. An
    /// empty slice encodes a null descriptor, so no pointer is taken from a
    /// zero-length slice. The buffer must stay valid until the request
    /// completes; the borrow itself only spans request construction.
    #[inline]
    pub fn add_in_slice(self, buffer: &[u8], mode: BufferMode) -> Self {
        let ptr = if buffer.is_empty() {
            ptr::null()
        } else {
            buffer.as_ptr()
        };
        self.add_in_buffer(ptr, buffer.len(), mode)
    }

    /// Adds a mapped output buffer (Type B) from a mutable byte slice.
    ///
    /// Slice-typed wrapper over [`add_out_buffer`](Self::add_out_buffer): the
    /// caller passes a borrowed buffer instead of a raw pointer and length. An
    /// empty slice encodes a null descriptor. The buffer must stay valid until
    /// the request completes; the borrow itself only spans request
    /// construction.
    #[inline]
    pub fn add_out_slice(self, buffer: &mut [u8], mode: BufferMode) -> Self {
        let ptr = if buffer.is_empty() {
            ptr::null_mut()
        } else {
            buffer.as_mut_ptr()
        };
        self.add_out_buffer(ptr, buffer.len(), mode)
    }

    /// Adds an exchange (in/out) buffer (Type W).
    #[inline]
    pub fn add_inout_buffer(mut self, buffer: *mut u8, size: usize, mode: BufferMode) -> Self {
        self.hipc = self
            .hipc
            .with_exch_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
        self
    }

    /// Adds an input pointer descriptor (Type X / send-static).
    #[inline]
    pub fn add_in_pointer(mut self, buffer: *const u8, size: usize) -> Self {
        let id = self.cur_in_ptr_id;
        self.hipc = self
            .hipc
            .with_send_static(StaticDescriptor::new_send(buffer, size, id));
        self.cur_in_ptr_id += 1;
        self.num_in_pointers += 1;
        self.server_pointer_size = self.server_pointer_size.saturating_sub(size);
        self
    }

    /// Adds a fixed-size output pointer (Type C / recv-list).
    #[inline]
    pub fn add_out_fixed_pointer(mut self, buffer: *mut u8, size: usize) -> Self {
        self.hipc = self
            .hipc
            .with_recv_list_entry(RecvListEntry::new_recv(buffer, size));
        self.num_out_fixed_pointers += 1;
        self.server_pointer_size = self.server_pointer_size.saturating_sub(size);
        self
    }

    /// Adds a variable-size output pointer (Type C with size tracked in OPT).
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

    /// Adds an auto-select input buffer.
    ///
    /// Uses an inline pointer if the buffer fits in the server's pointer
    /// buffer, otherwise falls back to a mapped buffer. In either case both
    /// a send-static and a send-buffer slot are reserved, with the unused
    /// side zeroed — preserving the wire layout pairing.
    #[inline]
    pub fn add_in_auto_buffer(self, buffer: *const u8, size: usize, mode: BufferMode) -> Self {
        let mut s = self;
        s.num_in_auto_buffers += 1;
        if s.server_pointer_size > 0 && size <= s.server_pointer_size {
            // Inline pointer + zero-filled buffer slot.
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
            // Zero-filled pointer slot + mapped buffer.
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

    /// Adds an auto-select output buffer (mirror of
    /// [`add_in_auto_buffer`](Self::add_in_auto_buffer) for OUT direction).
    #[inline]
    pub fn add_out_auto_buffer(self, buffer: *mut u8, size: usize, mode: BufferMode) -> Self {
        let mut s = self;
        s.num_out_auto_buffers += 1;
        if s.server_pointer_size > 0 && size <= s.server_pointer_size {
            // Inline output pointer + zero-filled recv-buffer slot.
            s.hipc = s
                .hipc
                .with_recv_list_entry(RecvListEntry::new_recv(buffer, size));
            s.hipc = s
                .hipc
                .with_recv_buffer(BufferDescriptor::new_buffer(ptr::null(), 0, mode));
            // OPT entry holds the actual size for the auto-pointer.
            s.out_pointer_sizes.push(size as u16);
            s.server_pointer_size = s.server_pointer_size.saturating_sub(size);
        } else {
            // Zero-filled recv-list slot + mapped recv-buffer.
            s.hipc = s
                .hipc
                .with_recv_list_entry(RecvListEntry::new_recv(ptr::null_mut(), 0));
            s.hipc = s
                .hipc
                .with_recv_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
            // OPT entry is zero in the mapped path.
            s.out_pointer_sizes.push(0);
        }
        s
    }

    /// Adds a domain input object id.
    #[inline]
    pub fn add_object(mut self, id: ObjectId) -> Self {
        self.objects.push(id.to_raw());
        self
    }

    /// Adds a copy handle.
    #[inline]
    pub fn add_copy_handle(mut self, handle: RawHandle) -> Self {
        self.hipc = self.hipc.with_copy_handle(handle);
        self
    }

    /// Adds a move handle.
    #[inline]
    pub fn add_move_handle(mut self, handle: RawHandle) -> Self {
        self.hipc = self.hipc.with_move_handle(handle);
        self
    }

    /// Finalizes the request DTO.
    pub fn build(self) -> CmifRequest<'a> {
        let message_type = if self.context != 0 {
            CommandType::RequestWithContext
        } else {
            CommandType::Request
        };
        let encoded_len = self.encoded_len();
        let hipc = self.hipc.set_message_type(message_type).build(encoded_len);
        CmifRequest {
            hipc,
            request_id: self.request_id,
            context: self.context,
            object_id: self.object_id,
            data: self.data,
            num_in_auto_buffers: self.num_in_auto_buffers,
            num_out_auto_buffers: self.num_out_auto_buffers,
            num_in_pointers: self.num_in_pointers,
            num_out_pointers: self.num_out_pointers,
            num_out_fixed_pointers: self.num_out_fixed_pointers,
            objects: self.objects,
            out_pointer_sizes: self.out_pointer_sizes,
        }
    }

    /// Writes the request into `dst` and returns the payload slice.
    #[inline]
    pub fn send<const N: usize>(self, dst: &mut [u8; N]) -> Result<&mut [u8], RequestLayoutError> {
        let req = self.build();
        let data_len = req.data_len();
        req.write_to(dst)?;
        let start = req.hipc.data_words_offset()
            + 16
            + if req.object_id.is_some() {
                size_of::<DomainInHeader>() + req.objects.len() * size_of::<u32>()
            } else {
                0
            }
            + size_of::<InHeader>();
        Ok(&mut dst[start..start + data_len])
    }

    fn encoded_len(&self) -> usize {
        let mut n: usize = 16;
        if self.object_id.is_some() {
            n += size_of::<DomainInHeader>() + self.objects.len() * size_of::<u32>();
        }
        n += size_of::<InHeader>() + self.data_len();
        n = (n + 1) & !1;
        n + size_of::<u16>() * (self.num_out_auto_buffers + self.num_out_pointers) as usize
    }
}

/// Parses a CMIF non-domain response message into a typed payload.
///
/// Validates the magic number and extracts the result code. On success,
/// returns a [`Response`] whose `payload` is a zerocopy view of `T` carved
/// out of the response buffer. The payload size is determined at compile
/// time from `size_of::<T>()`.
///
/// For empty payloads, use `T = ()`. For payloads whose size is only known
/// at runtime, use [`parse_response_bytes`]. For responses on a domain
/// session, use [`parse_response_domain`].
pub fn parse_response<'a, T>(
    buf: &'a [u8; IPC_BUFFER_SIZE],
) -> Result<Response<'a, T>, ParseRespError>
where
    T: FromBytes + Immutable + KnownLayout,
{
    let hipc_resp = hipc::parse_response(buf)?;

    let data_bytes: &'a [u8] = hipc_resp.data_words.as_bytes();

    let pad = data_bytes.as_ptr().align_offset(16);
    let (_padding, aligned) = data_bytes.split_at(pad);

    let (out_hdr_slot, rest) =
        OutHeader::ref_from_prefix(aligned).map_err(|_| ParseRespError::TruncatedOutHeader)?;
    let (payload, _) = T::ref_from_prefix(rest).map_err(|_| ParseRespError::TruncatedPayload)?;

    validate_out_header(out_hdr_slot)?;

    Ok(Response {
        payload,
        objects: &[],
        copy_handles: hipc_resp.copy_handles,
        move_handles: hipc_resp.move_handles,
    })
}

/// Parses a CMIF domain response message into a typed payload.
///
/// Same protocol-level validation as [`parse_response`], but also reads the
/// [`DomainOutHeader`] prefix and populates the returned object IDs. Use this
/// for responses received on a domain session.
pub fn parse_response_domain<'a, T>(
    buf: &'a [u8; IPC_BUFFER_SIZE],
) -> Result<Response<'a, T>, ParseRespError>
where
    T: FromBytes + Immutable + KnownLayout,
{
    let hipc_resp = hipc::parse_response(buf)?;

    let data_bytes: &'a [u8] = hipc_resp.data_words.as_bytes();

    let pad = data_bytes.as_ptr().align_offset(16);
    let (_padding, aligned) = data_bytes.split_at(pad);

    let (domain_hdr, rest) = DomainOutHeader::ref_from_prefix(aligned)
        .map_err(|_| ParseRespError::TruncatedDomainHeader)?;
    let (out_hdr_slot, rest) =
        OutHeader::ref_from_prefix(rest).map_err(|_| ParseRespError::TruncatedOutHeader)?;
    let (payload, rest) = T::ref_from_prefix(rest).map_err(|_| ParseRespError::TruncatedPayload)?;
    let count = domain_hdr.num_out_objects as usize;
    let (objects, _) = <[u32]>::ref_from_prefix_with_elems(rest, count)
        .map_err(|_| ParseRespError::TruncatedDomainObjects)?;

    validate_out_header(out_hdr_slot)?;

    Ok(Response {
        payload,
        objects,
        copy_handles: hipc_resp.copy_handles,
        move_handles: hipc_resp.move_handles,
    })
}

/// Validates the magic and result fields of a CMIF [`OutHeader`].
#[inline]
fn validate_out_header(hdr: &OutHeader) -> Result<(), ParseRespError> {
    if hdr.magic != OUT_HEADER_MAGIC {
        return Err(ParseRespError::InvalidMagic);
    }
    if hdr.result != 0 {
        return Err(ParseRespError::ServiceError(hdr.result));
    }
    Ok(())
}

/// Error returned by [`parse_response`].
#[derive(Debug, thiserror::Error)]
pub enum ParseRespError {
    /// Response contains invalid CMIF magic header.
    #[error("invalid CMIF magic header")]
    InvalidMagic,
    /// Service returned a non-zero result code.
    #[error("service error: {0:#x}")]
    ServiceError(u32),
    /// Underlying HIPC layer rejected the response.
    #[error("HIPC parse: {0}")]
    Hipc(#[from] hipc::ResponseParseError),
    /// Response too small to contain a CMIF `OutHeader`.
    #[error("CMIF response too small for OutHeader")]
    TruncatedOutHeader,
    /// Response too small to contain a CMIF `DomainOutHeader`.
    #[error("CMIF response too small for DomainOutHeader")]
    TruncatedDomainHeader,
    /// Response too small to contain the typed payload `T`.
    #[error("CMIF response too small for payload")]
    TruncatedPayload,
    /// Response too small to contain the domain object IDs.
    #[error("CMIF response too small for domain objects")]
    TruncatedDomainObjects,
}

/// Parses a CMIF non-domain response message with a runtime-sized byte payload.
///
/// Same protocol-level validation as [`parse_response`], but the payload
/// area is exposed as `&[u8]` of length `size`. Use this when the payload
/// size is only known at runtime (e.g. variable-length wire formats); for
/// fixed-shape payloads prefer the typed [`parse_response`]. For responses
/// on a domain session, use [`parse_response_bytes_domain`].
pub fn parse_response_bytes<'a>(
    buf: &'a [u8; IPC_BUFFER_SIZE],
    size: usize,
) -> Result<ResponseBytes<'a>, ParseRespBytesError> {
    let hipc_resp = hipc::parse_response(buf)?;

    let data_bytes: &'a [u8] = hipc_resp.data_words.as_bytes();

    let pad = data_bytes.as_ptr().align_offset(16);
    let (_padding, aligned) = data_bytes.split_at(pad);

    let (out_hdr_slot, rest) =
        OutHeader::ref_from_prefix(aligned).map_err(|_| ParseRespBytesError::TruncatedOutHeader)?;
    let (data, _) = rest
        .split_at_checked(size)
        .ok_or(ParseRespBytesError::TruncatedPayload)?;

    validate_out_header_bytes(out_hdr_slot)?;

    Ok(ResponseBytes {
        data,
        objects: &[],
        copy_handles: hipc_resp.copy_handles,
        move_handles: hipc_resp.move_handles,
    })
}

/// Parses a CMIF domain response message with a runtime-sized byte payload.
///
/// Same protocol-level validation as [`parse_response_bytes`], but also reads
/// the [`DomainOutHeader`] prefix and populates the returned object IDs.
pub fn parse_response_bytes_domain<'a>(
    buf: &'a [u8; IPC_BUFFER_SIZE],
    size: usize,
) -> Result<ResponseBytes<'a>, ParseRespBytesError> {
    let hipc_resp = hipc::parse_response(buf)?;

    let data_bytes: &'a [u8] = hipc_resp.data_words.as_bytes();

    let pad = data_bytes.as_ptr().align_offset(16);
    let (_padding, aligned) = data_bytes.split_at(pad);

    let (domain_hdr, rest) = DomainOutHeader::ref_from_prefix(aligned)
        .map_err(|_| ParseRespBytesError::TruncatedDomainHeader)?;
    let (out_hdr_slot, rest) =
        OutHeader::ref_from_prefix(rest).map_err(|_| ParseRespBytesError::TruncatedOutHeader)?;
    let (data, rest) = rest
        .split_at_checked(size)
        .ok_or(ParseRespBytesError::TruncatedPayload)?;
    let count = domain_hdr.num_out_objects as usize;
    let (objects, _) = <[u32]>::ref_from_prefix_with_elems(rest, count)
        .map_err(|_| ParseRespBytesError::TruncatedDomainObjects)?;

    validate_out_header_bytes(out_hdr_slot)?;

    Ok(ResponseBytes {
        data,
        objects,
        copy_handles: hipc_resp.copy_handles,
        move_handles: hipc_resp.move_handles,
    })
}

/// Validates the magic and result fields of a CMIF [`OutHeader`] for the
/// `parse_response_bytes*` family.
#[inline]
fn validate_out_header_bytes(hdr: &OutHeader) -> Result<(), ParseRespBytesError> {
    if hdr.magic != OUT_HEADER_MAGIC {
        return Err(ParseRespBytesError::InvalidMagic);
    }
    if hdr.result != 0 {
        return Err(ParseRespBytesError::ServiceError(hdr.result));
    }
    Ok(())
}

/// Error returned by [`parse_response_bytes`].
#[derive(Debug, thiserror::Error)]
pub enum ParseRespBytesError {
    /// Response contains invalid CMIF magic header.
    #[error("invalid CMIF magic header")]
    InvalidMagic,
    /// Service returned a non-zero result code.
    #[error("service error: {0:#x}")]
    ServiceError(u32),
    /// Underlying HIPC layer rejected the response.
    #[error("HIPC parse: {0}")]
    Hipc(#[from] hipc::ResponseParseError),
    /// Response too small to contain a CMIF `OutHeader`.
    #[error("CMIF response too small for OutHeader")]
    TruncatedOutHeader,
    /// Response too small to contain a CMIF `DomainOutHeader`.
    #[error("CMIF response too small for DomainOutHeader")]
    TruncatedDomainHeader,
    /// Response too small to contain the caller-requested payload size.
    #[error("CMIF response too small for payload")]
    TruncatedPayload,
    /// Response too small to contain the domain object IDs.
    #[error("CMIF response too small for domain objects")]
    TruncatedDomainObjects,
}

/// CMIF command type (stored in HIPC message type field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CommandType {
    /// Invalid command.
    Invalid = 0,
    /// Legacy request (pre-5.0.0).
    LegacyRequest = 1,
    /// Close session.
    Close = 2,
    /// Legacy control request.
    LegacyControl = 3,
    /// Standard request.
    Request = 4,
    /// Control request (domain conversion, cloning, etc.).
    Control = 5,
    /// Request with context token (5.0.0+).
    RequestWithContext = 6,
    /// Control request with context token.
    ControlWithContext = 7,
}

impl From<CommandType> for hipc::MessageType {
    fn from(cmd: CommandType) -> Self {
        hipc::MessageType::from_raw(cmd as u16)
    }
}

/// Domain request type (stored in domain header).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DomainRequestType {
    /// Invalid request.
    Invalid = 0,
    /// Send message to domain object.
    SendMessage = 1,
    /// Close domain object.
    Close = 2,
}

/// CMIF input header (16 bytes).
///
/// Present at the start of every CMIF request payload.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct InHeader {
    /// Magic number (`"SFCI"` = 0x49434653).
    pub magic: u32,
    /// Protocol version (0 = standard, 1 = with context).
    pub version: u32,
    /// Command/method ID to invoke.
    pub command_id: u32,
    /// Context token for versioning (non-domain only).
    pub token: u32,
}

const_assert_eq!(size_of::<InHeader>(), 16);

/// CMIF output header (16 bytes).
///
/// Present at the start of every CMIF response payload.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct OutHeader {
    /// Magic number (`"SFCO"` = 0x4F434653).
    pub magic: u32,
    /// Protocol version.
    pub version: u32,
    /// Result code (0 = success).
    pub result: u32,
    /// Echo of request token.
    pub token: u32,
}

const_assert_eq!(size_of::<OutHeader>(), 16);

/// Domain input header (16 bytes).
///
/// Prepended to CMIF header for domain requests.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct DomainInHeader {
    /// Request type (SendMessage or Close).
    pub request_type: u8,
    /// Number of object IDs in request.
    pub num_in_objects: u8,
    /// Size of CMIF header + payload.
    pub data_size: u16,
    /// Target object ID within domain.
    pub object_id: u32,
    /// Reserved padding.
    _padding: u32,
    /// Context token.
    pub token: u32,
}

const_assert_eq!(size_of::<DomainInHeader>(), 16);

/// Domain output header (16 bytes).
///
/// Prepended to CMIF header for domain responses.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct DomainOutHeader {
    /// Number of object IDs returned.
    pub num_out_objects: u32,
    /// Reserved padding.
    _padding: [u32; 3],
}

const_assert_eq!(size_of::<DomainOutHeader>(), 16);

/// Parsed CMIF response with a typed payload.
///
/// The payload is a zerocopy view of `T` carved out of the response buffer.
/// Use `T = ()` for responses with no payload data.
#[derive(Debug)]
pub struct Response<'a, T: ?Sized> {
    /// Typed response payload.
    pub payload: &'a T,
    /// Returned domain object IDs.
    pub objects: &'a [u32],
    /// Returned copy handles.
    pub copy_handles: &'a [RawHandle],
    /// Returned move handles.
    pub move_handles: &'a [RawHandle],
}

/// Parsed CMIF response with a raw byte payload.
///
/// Use this when the payload size is only known at runtime. For fixed-shape
/// payloads, prefer the typed [`Response`].
#[derive(Debug)]
pub struct ResponseBytes<'a> {
    /// Response payload data.
    pub data: &'a [u8],
    /// Returned domain object IDs.
    pub objects: &'a [u32],
    /// Returned copy handles.
    pub copy_handles: &'a [RawHandle],
    /// Returned move handles.
    pub move_handles: &'a [RawHandle],
}

/// A domain object identifier.
///
/// Identifies a specific service object within a CMIF domain session.
/// Object ID 0 is invalid; valid object IDs start at 1.
///
/// # Object IDs in CMIF Domains
///
/// When a service session is converted to a **domain**, it can multiplex
/// multiple service objects over a single IPC session handle. Each object
/// within the domain is identified by a unique 32-bit **Object ID**.
///
/// ## How Domains Work
///
/// Without domains, each service object requires its own kernel session handle.
/// This consumes kernel resources and limits scalability. Domains solve this by:
///
/// 1. Converting a session to a domain via `ConvertToDomain` control request
/// 2. The original service becomes object ID 1 within the domain
/// 3. Subsequent service objects acquired through this session get unique IDs
/// 4. All objects share the single underlying session handle
///
/// ## Message Format
///
/// In domain mode, CMIF requests include a [`DomainInHeader`] that specifies:
/// - The target object ID for the request
/// - Input object IDs being passed to the service
///
/// Responses include a [`DomainOutHeader`] with output object IDs.
///
/// ## Relationship to HIPC
///
/// Object IDs are a CMIF-layer concept built on top of HIPC:
///
/// ```text
/// ┌─────────────────────────────────────┐
/// │  Service (object_id = N)            │  ← ObjectId identifies target
/// ├─────────────────────────────────────┤
/// │  CMIF DomainInHeader { object_id }  │  ← ObjectId encoded here
/// ├─────────────────────────────────────┤
/// │  HIPC (session handle)              │  ← Single handle for all objects
/// └─────────────────────────────────────┘
/// ```
///
/// HIPC itself knows nothing about object IDs - it only deals with session
/// handles, buffer descriptors, and raw data. The CMIF layer adds the domain
/// abstraction on top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ObjectId(u32);

impl ObjectId {
    /// Creates an `ObjectId` from a raw value.
    ///
    /// Returns `None` if `raw` is zero, as zero is not a valid object ID.
    /// Valid object IDs start at 1 when a session is converted to a domain.
    #[inline]
    pub(crate) const fn new(raw: u32) -> Option<Self> {
        if raw == 0 { None } else { Some(Self(raw)) }
    }

    /// Creates an `ObjectId` from a raw value without validation.
    ///
    /// # Safety
    ///
    /// The caller must ensure the value is non-zero and represents a valid
    /// object ID obtained from the kernel (via `ConvertToDomain` or similar).
    #[inline]
    pub(crate) const unsafe fn new_unchecked(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw `u32` value of this object ID.
    #[inline]
    pub const fn to_raw(self) -> u32 {
        self.0
    }
}
