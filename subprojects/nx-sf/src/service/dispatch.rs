//! Fluent builders for dispatching CMIF requests.
//!
//! Two builder types match the operating mode of their originating wrapper:
//!
//! - [`Dispatch`] — produced by [`Session::dispatch`] and
//!   [`OverrideService::dispatch`]. Non-domain mode: the response cannot carry
//!   output domain objects, so there is no `out_objects` builder method and
//!   [`DispatchResult`] does not carry an `objects` field.
//! - [`DomainDispatch`] — produced by [`Domain::dispatch`] and
//!   [`DomainObject::dispatch`]. Domain mode: exposes [`out_objects`] and the
//!   resulting [`DomainDispatchResult`] hands back ready-to-use
//!   [`DomainObject`] instances, each constructed exactly once per dispatch
//!   from the server-emitted [`ObjectId`]s. This is the only safe path to
//!   obtain a [`DomainObject`] from outside the crate, which makes duplicate
//!   `DomainObject`s for the same id unrepresentable in safe code.
//!
//! [`Session`]: super::Session
//! [`OverrideService`]: super::OverrideService
//! [`Session::dispatch`]: super::Session::dispatch
//! [`Domain::dispatch`]: Domain::dispatch
//! [`DomainObject::dispatch`]: DomainObject::dispatch
//! [`OverrideService::dispatch`]: super::OverrideService::dispatch
//! [`out_objects`]: DomainDispatch::out_objects

use core::ptr;

use nx_svc::ipc::Handle as SessionHandle;
use nx_sys_thread_tls::IpcBuffer;

use super::domain::{Domain, DomainObject};
use crate::{
    cmif::{self, ObjectId},
    hipc, ipc,
};

/// Maximum number of buffers in a single dispatch.
pub const MAX_BUFFERS: usize = 8;

/// Maximum number of input objects in a single dispatch.
pub const MAX_IN_OBJECTS: usize = 8;

/// Maximum number of input handles in a single dispatch.
pub const MAX_IN_HANDLES: usize = 8;

/// Maximum number of output domain objects in a single dispatch.
pub const MAX_OUT_OBJECTS: usize = 8;

/// Buffer attribute flags for service dispatch.
#[derive(Debug, Clone, Copy, Default)]
pub struct BufferAttr(pub u32);

impl BufferAttr {
    /// Input buffer (data sent to service).
    pub const IN: Self = Self(1 << 0);
    /// Output buffer (data received from service).
    pub const OUT: Self = Self(1 << 1);
    /// Use HIPC MapAlias (Type A/B) buffer.
    pub const HIPC_MAP_ALIAS: Self = Self(1 << 2);
    /// Use HIPC Pointer (Type X/C) buffer.
    pub const HIPC_POINTER: Self = Self(1 << 3);
    /// Fixed size pointer buffer.
    pub const FIXED_SIZE: Self = Self(1 << 4);
    /// Auto-select between MapAlias and Pointer based on size.
    pub const HIPC_AUTO_SELECT: Self = Self(1 << 5);
    /// Allow non-secure transfer.
    pub const MAP_TRANSFER_ALLOWS_NON_SECURE: Self = Self(1 << 6);
    /// Allow non-device transfer.
    pub const MAP_TRANSFER_ALLOWS_NON_DEVICE: Self = Self(1 << 7);

    /// Combines two buffer attributes.
    #[inline]
    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Returns whether `flag` is set.
    #[inline]
    pub const fn contains(self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

/// Output handle attribute for service dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum OutHandleAttr {
    /// No handle expected.
    #[default]
    None = 0,
    /// Copy handle expected.
    Copy = 1,
    /// Move handle expected.
    Move = 2,
}

/// Buffer descriptor for dispatch.
#[derive(Debug, Clone, Copy, Default)]
pub struct Buffer {
    /// Pointer to buffer data.
    pub ptr: *const u8,
    /// Size of buffer in bytes.
    pub size: usize,
}

/// Builder for dispatching a single CMIF request.
#[derive(Debug)]
pub struct Dispatch<'a> {
    session: SessionHandle,
    pointer_buffer_size: u16,
    object_id: Option<ObjectId>,
    _borrow: core::marker::PhantomData<&'a ()>,
    request_id: u32,
    context: u32,
    in_data: *const u8,
    in_data_size: usize,
    out_data_size: usize,
    buffer_attrs: [BufferAttr; MAX_BUFFERS],
    buffers: [Buffer; MAX_BUFFERS],
    buffer_count: usize,
    in_objects: [Option<ObjectId>; MAX_IN_OBJECTS],
    in_object_count: usize,
    in_handles: [u32; MAX_IN_HANDLES],
    in_handle_count: usize,
    out_object_count: usize,
    out_handle_attrs: [OutHandleAttr; MAX_BUFFERS],
    send_pid: bool,
}

impl<'a> Dispatch<'a> {
    /// Creates a new dispatch builder. Used by the typed wrappers.
    #[inline]
    pub(crate) fn new(
        session: SessionHandle,
        pointer_buffer_size: u16,
        object_id: Option<ObjectId>,
        request_id: u32,
    ) -> Self {
        Self {
            session,
            pointer_buffer_size,
            object_id,
            _borrow: core::marker::PhantomData,
            request_id,
            context: 0,
            in_data: ptr::null(),
            in_data_size: 0,
            out_data_size: 0,
            buffer_attrs: [BufferAttr::default(); MAX_BUFFERS],
            buffers: [Buffer::default(); MAX_BUFFERS],
            buffer_count: 0,
            in_objects: [None; MAX_IN_OBJECTS],
            in_object_count: 0,
            in_handles: [0; MAX_IN_HANDLES],
            in_handle_count: 0,
            out_object_count: 0,
            out_handle_attrs: [OutHandleAttr::None; MAX_BUFFERS],
            send_pid: false,
        }
    }

    /// Sets the context token for versioning.
    #[inline]
    pub fn context(mut self, context: u32) -> Self {
        self.context = context;
        self
    }

    /// Sets the input data for the request. The slice must remain valid
    /// until [`send`](Self::send) returns; the borrow is enforced via `'a`.
    #[inline]
    pub fn in_raw(mut self, data: &'a [u8]) -> Self {
        self.in_data = data.as_ptr();
        self.in_data_size = data.len();
        self
    }

    /// Sets the expected output data size.
    #[inline]
    pub fn out_size(mut self, size: usize) -> Self {
        self.out_data_size = size;
        self
    }

    /// Adds an IN buffer. [`BufferAttr::IN`] is set automatically; the
    /// caller supplies only the transport bits (e.g. [`BufferAttr::HIPC_MAP_ALIAS`],
    /// [`BufferAttr::HIPC_POINTER`], [`BufferAttr::HIPC_AUTO_SELECT`],
    /// [`BufferAttr::FIXED_SIZE`]). Silently ignored once [`MAX_BUFFERS`]
    /// slots are full.
    #[inline]
    pub fn in_buffer(self, data: &'a [u8], attr: BufferAttr) -> Self {
        self.push_buffer(data.as_ptr(), data.len(), attr.or(BufferAttr::IN))
    }

    /// Adds an OUT buffer. [`BufferAttr::OUT`] is set automatically.
    /// Silently ignored once [`MAX_BUFFERS`] slots are full.
    #[inline]
    pub fn out_buffer(self, data: &'a mut [u8], attr: BufferAttr) -> Self {
        self.push_buffer(data.as_mut_ptr(), data.len(), attr.or(BufferAttr::OUT))
    }

    /// Adds an IN/OUT buffer. Both [`BufferAttr::IN`] and [`BufferAttr::OUT`]
    /// are set automatically. Silently ignored once [`MAX_BUFFERS`] slots
    /// are full.
    #[inline]
    pub fn inout_buffer(self, data: &'a mut [u8], attr: BufferAttr) -> Self {
        self.push_buffer(
            data.as_mut_ptr(),
            data.len(),
            attr.or(BufferAttr::IN).or(BufferAttr::OUT),
        )
    }

    /// Records a buffer descriptor in the internal table. Silently ignored
    /// once [`MAX_BUFFERS`] slots are full.
    #[inline]
    fn push_buffer(mut self, ptr: *const u8, size: usize, attr: BufferAttr) -> Self {
        if self.buffer_count < MAX_BUFFERS {
            self.buffers[self.buffer_count] = Buffer { ptr, size };
            self.buffer_attrs[self.buffer_count] = attr;
            self.buffer_count += 1;
        }
        self
    }

    /// Adds an input domain object. Silently ignored once
    /// [`MAX_IN_OBJECTS`] slots are full.
    #[inline]
    pub fn in_object(mut self, object_id: ObjectId) -> Self {
        if self.in_object_count < MAX_IN_OBJECTS {
            self.in_objects[self.in_object_count] = Some(object_id);
            self.in_object_count += 1;
        }
        self
    }

    /// Adds an input handle. Silently ignored once [`MAX_IN_HANDLES`] slots
    /// are full.
    #[inline]
    pub fn in_handle(mut self, handle: u32) -> Self {
        if self.in_handle_count < MAX_IN_HANDLES {
            self.in_handles[self.in_handle_count] = handle;
            self.in_handle_count += 1;
        }
        self
    }

    /// Sets an output handle attribute at the given index. Out-of-range
    /// indices are silently ignored.
    #[inline]
    pub fn out_handle(mut self, index: usize, attr: OutHandleAttr) -> Self {
        if index < MAX_BUFFERS {
            self.out_handle_attrs[index] = attr;
        }
        self
    }

    /// Enables sending the process ID alongside the request.
    #[inline]
    pub fn send_pid(mut self) -> Self {
        self.send_pid = true;
        self
    }

    /// Sends the dispatch request and parses the response.
    ///
    /// The returned [`DispatchResult`] borrows from `buf` (the per-thread
    /// IPC TLS buffer). The borrow's lifetime ties response references to
    /// the caller-provided token, so the borrow checker enforces that the
    /// response is fully consumed before the next IPC operation reuses
    /// `buf`.
    pub fn send<'b>(self, buf: &'b mut IpcBuffer) -> Result<DispatchResult<'b>, DispatchError> {
        let resp = self.send_response(buf)?;
        Ok(DispatchResult {
            data: resp.data,
            copy_handles: resp.copy_handles,
            move_handles: resp.move_handles,
        })
    }

    /// Sends the dispatch request and returns the raw CMIF response.
    ///
    /// Crate-internal: used by [`Dispatch::send`] for the non-domain path and
    /// by [`DomainDispatch::send`] to wrap the response's raw object ids into
    /// [`DomainObject`] instances before exposing them to callers.
    pub(crate) fn send_response<'b>(
        self,
        buf: &'b mut IpcBuffer,
    ) -> Result<cmif::ResponseBytes<'b>, DispatchError> {
        let is_domain = self.object_id.is_some();

        {
            let mut cb = cmif::CmifRequestBuilder::new(self.request_id)
                .with_pointer_buffer_size(self.pointer_buffer_size as usize)
                .with_context(self.context);

            let in_slice: &[u8] = if !self.in_data.is_null() && self.in_data_size > 0 {
                // SAFETY: caller of `in_raw` guarantees `in_data` is valid for
                // `in_data_size` bytes.
                unsafe { core::slice::from_raw_parts(self.in_data, self.in_data_size) }
            } else {
                &[]
            };
            cb = cb.with_data(in_slice);

            if let Some(object_id) = self.object_id {
                cb = cb.with_object_id(object_id);
            }
            if self.send_pid {
                cb = cb.with_send_pid();
            }

            for i in 0..self.buffer_count {
                let buf_desc = &self.buffers[i];
                let attr = self.buffer_attrs[i];
                let is_in = attr.contains(BufferAttr::IN);
                let is_out = attr.contains(BufferAttr::OUT);

                let mode = if attr.contains(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE) {
                    hipc::BufferMode::NonSecure
                } else if attr.contains(BufferAttr::MAP_TRANSFER_ALLOWS_NON_DEVICE) {
                    hipc::BufferMode::NonDevice
                } else {
                    hipc::BufferMode::Normal
                };

                if attr.contains(BufferAttr::HIPC_AUTO_SELECT) {
                    if is_in {
                        cb = cb.add_in_auto_buffer(buf_desc.ptr, buf_desc.size, mode);
                    }
                    if is_out {
                        cb = cb.add_out_auto_buffer(buf_desc.ptr as *mut u8, buf_desc.size, mode);
                    }
                } else if attr.contains(BufferAttr::HIPC_MAP_ALIAS) {
                    if is_in && is_out {
                        cb = cb.add_inout_buffer_raw(buf_desc.ptr as *mut u8, buf_desc.size, mode);
                    } else if is_in {
                        cb = cb.add_input_buffer_raw(buf_desc.ptr, buf_desc.size, mode);
                    } else if is_out {
                        cb = cb.add_output_buffer_raw(buf_desc.ptr as *mut u8, buf_desc.size, mode);
                    }
                } else if attr.contains(BufferAttr::HIPC_POINTER) {
                    if is_in {
                        cb = cb.add_in_pointer(buf_desc.ptr, buf_desc.size);
                    } else if is_out {
                        if attr.contains(BufferAttr::FIXED_SIZE) {
                            cb = cb.add_out_fixed_pointer(buf_desc.ptr as *mut u8, buf_desc.size);
                        } else {
                            cb = cb.add_out_pointer(buf_desc.ptr as *mut u8, buf_desc.size);
                        }
                    }
                }
            }

            for i in 0..self.in_object_count {
                if let Some(obj) = self.in_objects[i] {
                    cb = cb.add_object(obj);
                }
            }

            for i in 0..self.in_handle_count {
                cb = cb.add_copy_handle(self.in_handles[i]);
            }

            let req = cb.build();
            req.write_to(buf).map_err(DispatchError::Layout)?;
        }

        // Reborrows `buf` uniquely: the borrow checker invalidates any
        // outstanding `&[u8; N]` derived from `buf` (e.g. from the request
        // builder above) before the syscall runs. After this call returns,
        // fresh borrows from `buf` observe the kernel-written response.
        ipc::send_sync_request(buf, self.session).map_err(DispatchError::SendRequest)?;

        // Response bytes borrow from `buf`; lifetime `'b` propagates to the
        // caller, who must consume the response before the next IPC
        // operation reuses the token.
        let resp = if is_domain {
            cmif::parse_response_bytes_domain(buf.as_array(), self.out_data_size)
        } else {
            cmif::parse_response_bytes(buf.as_array(), self.out_data_size)
        }
        .map_err(DispatchError::ParseResponse)?;

        Ok(resp)
    }
}

/// Error returned by [`Dispatch::send`].
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    /// The request does not fit in the IPC buffer.
    #[error("IPC request layout error")]
    Layout(#[source] cmif::RequestLayoutError),
    /// The kernel rejected the underlying `SendSyncRequest`.
    #[error("failed to send IPC request")]
    SendRequest(#[source] nx_svc::ipc::SendSyncError),
    /// The response header did not pass CMIF validation.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Result of a successful non-domain dispatch operation.
///
/// Non-domain sessions cannot receive output domain objects, so this type
/// carries no `objects` field. Domain dispatches use
/// [`DomainDispatchResult`] instead.
#[derive(Debug)]
pub struct DispatchResult<'a> {
    /// Response payload data.
    pub data: &'a [u8],
    /// Returned copy handles.
    pub copy_handles: &'a [u32],
    /// Returned move handles.
    pub move_handles: &'a [u32],
}

/// Builder for dispatching a single CMIF request in domain mode.
///
/// Wraps a [`Dispatch`] together with a borrow of the parent [`Domain`].
/// The borrow lets [`send`](Self::send) construct [`DomainObject<'d>`]
/// instances from the server-emitted object ids in the response, so the
/// caller never has to launder raw `u32` ids back through an unsafe
/// constructor.
#[derive(Debug)]
pub struct DomainDispatch<'d> {
    inner: Dispatch<'d>,
    domain: &'d Domain,
}

impl<'d> DomainDispatch<'d> {
    /// Creates a new domain-dispatch builder. Used by the typed wrappers.
    #[inline]
    pub(crate) fn new(
        domain: &'d Domain,
        session: SessionHandle,
        pointer_buffer_size: u16,
        object_id: Option<ObjectId>,
        request_id: u32,
    ) -> Self {
        Self {
            inner: Dispatch::new(session, pointer_buffer_size, object_id, request_id),
            domain,
        }
    }

    /// Sets the context token for versioning.
    #[inline]
    pub fn context(mut self, context: u32) -> Self {
        self.inner = self.inner.context(context);
        self
    }

    /// Sets the input data for the request. The slice must remain valid
    /// until [`send`](Self::send) returns; the borrow is enforced via `'d`.
    #[inline]
    pub fn in_raw(mut self, data: &'d [u8]) -> Self {
        self.inner = self.inner.in_raw(data);
        self
    }

    /// Sets the expected output data size.
    #[inline]
    pub fn out_size(mut self, size: usize) -> Self {
        self.inner = self.inner.out_size(size);
        self
    }

    /// Adds an IN buffer. [`BufferAttr::IN`] is set automatically; the
    /// caller supplies only the transport bits. Silently ignored once
    /// [`MAX_BUFFERS`] slots are full.
    #[inline]
    pub fn in_buffer(mut self, data: &'d [u8], attr: BufferAttr) -> Self {
        self.inner = self.inner.in_buffer(data, attr);
        self
    }

    /// Adds an OUT buffer. [`BufferAttr::OUT`] is set automatically.
    /// Silently ignored once [`MAX_BUFFERS`] slots are full.
    #[inline]
    pub fn out_buffer(mut self, data: &'d mut [u8], attr: BufferAttr) -> Self {
        self.inner = self.inner.out_buffer(data, attr);
        self
    }

    /// Adds an IN/OUT buffer. Both [`BufferAttr::IN`] and [`BufferAttr::OUT`]
    /// are set automatically. Silently ignored once [`MAX_BUFFERS`] slots
    /// are full.
    #[inline]
    pub fn inout_buffer(mut self, data: &'d mut [u8], attr: BufferAttr) -> Self {
        self.inner = self.inner.inout_buffer(data, attr);
        self
    }

    /// Adds an input domain object. Silently ignored once
    /// [`MAX_IN_OBJECTS`] slots are full.
    #[inline]
    pub fn in_object(mut self, object_id: ObjectId) -> Self {
        self.inner = self.inner.in_object(object_id);
        self
    }

    /// Adds an input handle. Silently ignored once [`MAX_IN_HANDLES`] slots
    /// are full.
    #[inline]
    pub fn in_handle(mut self, handle: u32) -> Self {
        self.inner = self.inner.in_handle(handle);
        self
    }

    /// Sets the number of output domain objects expected.
    ///
    /// # Panics
    ///
    /// Panics if `count` exceeds [`MAX_OUT_OBJECTS`]. Requesting more
    /// objects than the protocol cap is a programming error; silent
    /// clamping would mask a server-side leak of the unwrapped extras.
    #[inline]
    pub fn out_objects(mut self, count: usize) -> Self {
        assert!(
            count <= MAX_OUT_OBJECTS,
            "out_objects: requested count exceeds MAX_OUT_OBJECTS",
        );
        self.inner.out_object_count = count;
        self
    }

    /// Sets an output handle attribute at the given index. Out-of-range
    /// indices are silently ignored.
    #[inline]
    pub fn out_handle(mut self, index: usize, attr: OutHandleAttr) -> Self {
        self.inner = self.inner.out_handle(index, attr);
        self
    }

    /// Enables sending the process ID alongside the request.
    #[inline]
    pub fn send_pid(mut self) -> Self {
        self.inner = self.inner.send_pid();
        self
    }

    /// Sends the dispatch request and parses the response into a
    /// [`DomainDispatchResult`] holding freshly-constructed
    /// [`DomainObject<'d>`] instances bound to the originating
    /// [`Domain`]. Each server-emitted [`ObjectId`] becomes exactly one
    /// [`DomainObject`].
    pub fn send<'b>(
        self,
        buf: &'b mut IpcBuffer,
    ) -> Result<DomainDispatchResult<'d, 'b>, DispatchError> {
        let domain = self.domain;
        let resp = self.inner.send_response(buf)?;

        // `resp.objects.len()` is bounded by the server's response, which
        // honors the request's `out_object_count` (capped at
        // `MAX_OUT_OBJECTS` by `DomainDispatch::out_objects`). The
        // `MAX_OUT_OBJECTS` zip bound below is a defensive guard, not a
        // silent truncation point.
        let mut objects: [Option<DomainObject<'d>>; MAX_OUT_OBJECTS] =
            [const { None }; MAX_OUT_OBJECTS];
        let mut object_count = 0;
        for (slot, &raw) in objects.iter_mut().zip(resp.objects.iter()) {
            if let Some(id) = ObjectId::new(raw) {
                *slot = Some(domain.open_object(id));
                object_count += 1;
            }
        }

        Ok(DomainDispatchResult {
            data: resp.data,
            copy_handles: resp.copy_handles,
            move_handles: resp.move_handles,
            objects,
            object_count,
        })
    }
}

/// Result of a successful domain dispatch operation.
///
/// Holds up to [`MAX_OUT_OBJECTS`] freshly-issued [`DomainObject<'d>`]s.
/// The lifetime `'d` ties each `DomainObject` to the parent [`Domain`].
/// Use [`take_object`](Self::take_object) to claim a specific slot or
/// [`into_objects`](Self::into_objects) to consume them in order;
/// objects left in the result at drop time are closed normally.
#[derive(Debug)]
pub struct DomainDispatchResult<'d, 'b> {
    /// Response payload data. Borrows from the IPC TLS buffer (`'b`).
    pub data: &'b [u8],
    /// Returned copy handles. Borrows from the IPC TLS buffer (`'b`).
    pub copy_handles: &'b [u32],
    /// Returned move handles. Borrows from the IPC TLS buffer (`'b`).
    pub move_handles: &'b [u32],
    objects: [Option<DomainObject<'d>>; MAX_OUT_OBJECTS],
    object_count: usize,
}

impl<'d, 'b> DomainDispatchResult<'d, 'b> {
    /// Number of [`DomainObject`]s the server emitted in this response.
    #[inline]
    pub fn object_count(&self) -> usize {
        self.object_count
    }

    /// Takes the [`DomainObject`] in slot `idx`, leaving `None` behind.
    /// Returns `None` for empty or already-taken slots and for any
    /// `idx >= MAX_OUT_OBJECTS`.
    #[inline]
    pub fn take_object(&mut self, idx: usize) -> Option<DomainObject<'d>> {
        self.objects.get_mut(idx).and_then(|slot| slot.take())
    }

    /// Consumes the result and yields the populated [`DomainObject`]s
    /// in slot order. Slots already drained via
    /// [`take_object`](Self::take_object) are skipped.
    #[inline]
    pub fn into_objects(self) -> impl Iterator<Item = DomainObject<'d>> + 'd {
        self.objects.into_iter().flatten()
    }
}
