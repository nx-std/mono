//! CMIF request building.
//!
//! This module contains request values and fluent builders for CMIF
//! requests. Serialization and response parsing live in sibling modules.

use core::{convert::Infallible, mem::size_of, ptr};

use nx_svc::raw::Handle as RawHandle;
use zerocopy::FromBytes;

use super::{
    object_id::ObjectId,
    wire::{CMIF_MAX_OBJECTS, CommandType, DomainInHeader, DomainRequestType, InHeader},
};
use crate::hipc::{
    self, ArrayVec, BufferDescriptor, BufferMode, HIPC_MAX_RECV_LIST, HipcRequest,
    HipcRequestBuilder, RecvListEntry, StaticDescriptor,
};

/// Layout error for CMIF request serialization.
///
/// CMIF request writers cannot fail while encoding their own headers; layout
/// failures come from the underlying HIPC request size check.
pub type RequestLayoutError = hipc::BuildError<Infallible>;

/// Value-type description of a full CMIF request body.
///
/// Holds the HIPC envelope plus the CMIF headers, payload sizing, domain object
/// IDs, and out-pointer-size table needed to serialize a request. Most callers
/// create this through [`CmifRequestBuilder`].
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
            magic: super::wire::IN_HEADER_MAGIC,
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

/// CMIF control request body.
///
/// Control requests use the CMIF control command type for operations such as
/// domain conversion, cloning, and pointer-buffer-size queries.
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
            magic: super::wire::IN_HEADER_MAGIC,
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
///
/// Accumulates a control command ID and optional payload before producing a
/// control request or writing it directly into an IPC buffer.
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

    /// Finalizes the request value.
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

/// CMIF close request body.
///
/// Represents either a plain session close or a domain-object close message.
#[derive(Debug, Clone)]
pub enum CmifCloseRequest {
    /// Session close.
    Session(HipcRequest),
    /// Domain object close.
    DomainObject {
        /// HIPC request envelope used to carry the domain close header.
        hipc: HipcRequest,
        /// Domain object to close on the shared session.
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

    /// Sets the server pointer-buffer capacity used by auto-buffer selection.
    ///
    /// Auto-buffers use inline pointer descriptors while enough pointer-buffer
    /// capacity remains, then fall back to mapped buffer descriptors.
    #[inline]
    pub fn pointer_buffer_size(mut self, size: usize) -> Self {
        self.server_pointer_size = size;
        self
    }

    /// Sets the context token for versioned requests.
    ///
    /// Non-zero values switch the HIPC message type to
    /// [`CommandType::RequestWithContext`] and set the CMIF header version to
    /// 1.
    #[inline]
    pub fn context(mut self, ctx: u32) -> Self {
        self.context = ctx;
        self
    }

    /// Sets the request payload bytes to copy into the CMIF data area.
    #[inline]
    pub fn data(mut self, data: &'a [u8]) -> Self {
        self.data = RequestData::Borrowed(data);
        self
    }

    /// Reserves a payload area of `size` bytes for the caller to fill after send.
    #[inline]
    pub fn data_size(mut self, size: usize) -> Self {
        self.data = RequestData::Sized(size);
        self
    }

    /// Marks the request as targeting an object inside a CMIF domain.
    #[inline]
    pub fn object_id(mut self, id: ObjectId) -> Self {
        self.object_id = Some(id);
        self
    }

    /// Includes the current process ID in the underlying HIPC request.
    #[inline]
    pub fn send_pid(mut self) -> Self {
        self.hipc = self.hipc.with_send_pid();
        self
    }

    /// Adds a mapped input buffer using a Type-A HIPC descriptor.
    #[inline]
    pub fn add_in_buffer(mut self, buffer: *const u8, size: usize, mode: BufferMode) -> Self {
        self.hipc = self
            .hipc
            .with_send_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
        self
    }

    /// Adds a mapped output buffer using a Type-B HIPC descriptor.
    #[inline]
    pub fn add_out_buffer(mut self, buffer: *mut u8, size: usize, mode: BufferMode) -> Self {
        self.hipc = self
            .hipc
            .with_recv_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
        self
    }

    /// Adds a mapped input buffer from a byte slice.
    ///
    /// Empty slices encode a null descriptor instead of taking a dangling
    /// pointer from the slice.
    #[inline]
    pub fn add_in_slice(self, buffer: &[u8], mode: BufferMode) -> Self {
        let ptr = if buffer.is_empty() {
            ptr::null()
        } else {
            buffer.as_ptr()
        };
        self.add_in_buffer(ptr, buffer.len(), mode)
    }

    /// Adds a mapped output buffer from a mutable byte slice.
    ///
    /// Empty slices encode a null descriptor instead of taking a dangling
    /// pointer from the slice.
    #[inline]
    pub fn add_out_slice(self, buffer: &mut [u8], mode: BufferMode) -> Self {
        let ptr = if buffer.is_empty() {
            ptr::null_mut()
        } else {
            buffer.as_mut_ptr()
        };
        self.add_out_buffer(ptr, buffer.len(), mode)
    }

    /// Adds a mapped bidirectional buffer using a Type-W HIPC descriptor.
    #[inline]
    pub fn add_inout_buffer(mut self, buffer: *mut u8, size: usize, mode: BufferMode) -> Self {
        self.hipc = self
            .hipc
            .with_exch_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
        self
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
        self.num_in_pointers += 1;
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
        self.num_out_fixed_pointers += 1;
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
        s.num_in_auto_buffers += 1;
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
        self.objects.push(id.to_raw());
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

#[derive(Debug, Clone, Copy)]
enum RequestData<'a> {
    Borrowed(&'a [u8]),
    Sized(usize),
}
