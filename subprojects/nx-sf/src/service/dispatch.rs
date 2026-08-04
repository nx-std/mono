//! Fluent builders for dispatching CMIF requests.
//!
//! Two builder types match the operating mode of their originating wrapper:
//!
//! - [`Dispatch`] - produced by [`Session::dispatch`] and
//!   [`OverrideService::dispatch`]. Non-domain mode: the response cannot carry
//!   output domain objects, so there is no `out_objects` builder method and
//!   [`DispatchResult`] does not carry an `objects` field.
//! - [`DomainDispatch`] - produced by [`Domain::dispatch`] and
//!   [`DomainObjectRef::dispatch`]. Domain mode: exposes [`out_objects`] and
//!   the resulting [`DomainDispatchResult`] hands back ready-to-use
//!   [`DomainObject`] instances, each constructed exactly once per dispatch
//!   from the server-emitted [`ObjectId`]s. Every id the server issues acquires
//!   its owner here, which is what makes each one closed exactly once.
//!
//! [`Session`]: super::Session
//! [`OverrideService`]: super::OverrideService
//! [`Session::dispatch`]: super::Session::dispatch
//! [`Domain`]: super::Domain
//! [`Domain::dispatch`]: super::Domain::dispatch
//! [`DomainObjectRef`]: super::DomainObjectRef
//! [`DomainObjectRef::dispatch`]: super::DomainObjectRef::dispatch
//! [`OverrideService::dispatch`]: super::OverrideService::dispatch
//! [`out_objects`]: DomainDispatch::out_objects

use nx_svc::error::{
    ResultCode,
    ToResultCode as _,
};
use nx_sys_thread_tls::IpcBuffer;

use super::domain::{
    DomainObject,
    DomainRef,
};
use crate::{
    cmif::{
        self,
        ObjectId,
    },
    error::ToResultCode,
    hipc::{
        self,
        InOutBuffer,
        InPointer,
        InputBuffer,
        OutPointer,
        OutputBuffer,
    },
    service::handle::BorrowedSessionHandle,
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
pub struct BufferAttr(u32);

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

/// Builder for dispatching a single CMIF request.
#[derive(Debug)]
pub struct Dispatch<'a> {
    session: BorrowedSessionHandle<'a>,
    pointer_buffer_size: u16,
    object_id: Option<ObjectId>,
    request_id: u32,
    context: u32,
    in_data: &'a [u8],
    out_data_size: usize,
    buffers: [Option<(BufferAttr, BufferSlot<'a>)>; MAX_BUFFERS],
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
        session: BorrowedSessionHandle<'a>,
        pointer_buffer_size: u16,
        object_id: Option<ObjectId>,
        request_id: u32,
    ) -> Self {
        Self {
            session,
            pointer_buffer_size,
            object_id,
            request_id,
            context: 0,
            in_data: &[],
            out_data_size: 0,
            buffers: [const { None }; MAX_BUFFERS],
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

    /// Sets the input data for the request. The borrow is held until
    /// [`send`](Self::send) returns.
    #[inline]
    pub fn in_raw(mut self, data: &'a [u8]) -> Self {
        self.in_data = data;
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
        self.push_buffer(BufferSlot::In(data), attr.or(BufferAttr::IN))
    }

    /// Adds an OUT buffer. [`BufferAttr::OUT`] is set automatically.
    /// Silently ignored once [`MAX_BUFFERS`] slots are full.
    #[inline]
    pub fn out_buffer(self, data: &'a mut [u8], attr: BufferAttr) -> Self {
        self.push_buffer(BufferSlot::Out(data), attr.or(BufferAttr::OUT))
    }

    /// Adds an IN/OUT buffer. Both [`BufferAttr::IN`] and [`BufferAttr::OUT`]
    /// are set automatically. Silently ignored once [`MAX_BUFFERS`] slots
    /// are full.
    #[inline]
    pub fn inout_buffer(self, data: &'a mut [u8], attr: BufferAttr) -> Self {
        self.push_buffer(
            BufferSlot::InOut(data),
            attr.or(BufferAttr::IN).or(BufferAttr::OUT),
        )
    }

    /// Records a buffer slot in the internal table. Silently ignored once
    /// [`MAX_BUFFERS`] slots are full.
    #[inline]
    fn push_buffer(mut self, slot: BufferSlot<'a>, attr: BufferAttr) -> Self {
        if self.buffer_count < MAX_BUFFERS {
            self.buffers[self.buffer_count] = Some((attr, slot));
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
    ///
    /// # Panics
    ///
    /// Panics in debug builds if a buffer registered via
    /// [`inout_buffer`](Self::inout_buffer) carries
    /// [`BufferAttr::HIPC_POINTER`]; the combination is unsupported and the
    /// slot is skipped in release builds.
    pub fn send<'b>(self, buf: &'b mut IpcBuffer) -> Result<DispatchResult<'b>, DispatchError> {
        let resp = self.send_response(buf)?;
        Ok(DispatchResult {
            data: resp.payload,
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
    ) -> Result<cmif::Response<'b, &'b [u8]>, DispatchError> {
        let is_domain = self.object_id.is_some();

        let mut cb = cmif::CmifRequestBuilder::new(self.request_id)
            .with_pointer_buffer_size(self.pointer_buffer_size)
            .with_context(self.context)
            .with_data(self.in_data);

        if let Some(object_id) = self.object_id {
            cb = cb.with_object_id(object_id);
        }
        if self.send_pid {
            cb = cb.with_send_pid();
        }

        // Each slot moves its loan into the matching builder wrapper; the
        // request DTO then holds every borrow until `send` returns.
        for (attr, slot) in self.buffers.into_iter().flatten() {
            let mode = if attr.contains(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE) {
                hipc::BufferMode::NonSecure
            } else if attr.contains(BufferAttr::MAP_TRANSFER_ALLOWS_NON_DEVICE) {
                hipc::BufferMode::NonDevice
            } else {
                hipc::BufferMode::Normal
            };

            if attr.contains(BufferAttr::HIPC_AUTO_SELECT) {
                cb = match slot {
                    BufferSlot::In(data) => cb.add_in_auto_buffer(InputBuffer::new(data, mode)),
                    BufferSlot::Out(data) => cb.add_out_auto_buffer(OutputBuffer::new(data, mode)),
                    BufferSlot::InOut(data) => {
                        cb.add_inout_auto_buffer(InOutBuffer::new(data, mode))
                    }
                };
            } else if attr.contains(BufferAttr::HIPC_MAP_ALIAS) {
                cb = match slot {
                    BufferSlot::In(data) => cb.add_input_buffer(InputBuffer::new(data, mode)),
                    BufferSlot::Out(data) => cb.add_output_buffer(OutputBuffer::new(data, mode)),
                    BufferSlot::InOut(data) => cb.add_inout_buffer(InOutBuffer::new(data, mode)),
                };
            } else if attr.contains(BufferAttr::HIPC_POINTER) {
                cb = match slot {
                    BufferSlot::In(data) => cb.add_in_pointer(InPointer::new(data)),
                    BufferSlot::Out(data) => {
                        if attr.contains(BufferAttr::FIXED_SIZE) {
                            cb.add_out_fixed_pointer(OutPointer::new(data))
                        } else {
                            cb.add_out_pointer(OutPointer::new(data))
                        }
                    }
                    BufferSlot::InOut(_) => {
                        // libnx encodes this as an aliased X + C pair over
                        // the same memory, which one exclusive loan cannot
                        // express. No service uses the combination; reject
                        // it loudly in debug builds and skip the slot
                        // otherwise.
                        debug_assert!(false, "inout_buffer with HIPC_POINTER is not supported");
                        cb
                    }
                };
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

        // The consuming `send` holds every buffer loan across the syscall
        // and uniquely reborrows `buf`, so no stale borrow of the TLS bytes
        // survives into the kernel's response write.
        cb.build()
            .send(buf, self.session)
            .map_err(|err| match err {
                cmif::SendError::Layout(err) => DispatchError::Layout(err),
                cmif::SendError::SendRequest(err) => DispatchError::SendRequest(err),
            })?;

        // Response bytes borrow from `buf`; lifetime `'b` propagates to the
        // caller, who must consume the response before the next IPC
        // operation reuses the token.
        let resp = if is_domain {
            cmif::parse_response_domain_bytes(buf.as_array(), self.out_data_size)
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
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for DispatchError {
    fn to_rc(self) -> ResultCode {
        match self {
            DispatchError::Layout(err) => err.to_rc(),
            // The kernel owns this code, so it resolves through `nx-svc`'s
            // trait rather than this crate's.
            DispatchError::SendRequest(err) => err.to_rc(),
            DispatchError::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Borrowed buffer slot recorded by the dispatch builder.
///
/// The variant fixes the transfer direction, so the kernel-facing role of
/// every buffer is carried by a real loan (`&`/`&mut`) instead of a raw
/// pointer. The paired [`BufferAttr`] supplies only the transport bits
/// (map-alias / pointer / auto-select, mode, fixed-size).
#[derive(Debug)]
enum BufferSlot<'a> {
    /// Kernel-read buffer registered via [`Dispatch::in_buffer`].
    In(&'a [u8]),
    /// Kernel-written buffer registered via [`Dispatch::out_buffer`].
    Out(&'a mut [u8]),
    /// Kernel-read-then-written buffer registered via
    /// [`Dispatch::inout_buffer`].
    InOut(&'a mut [u8]),
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

impl<'a> DispatchResult<'a> {
    /// Borrows the response payload as a typed reference.
    ///
    /// Infallible by construction: the caller of [`Dispatch::out_size`]
    /// declares `size_of::<T>()`, and the underlying CMIF parser yields a
    /// 16-byte-aligned `data` slice - sufficient for any `T` whose
    /// `align_of` is ≤ 16. Panics only if those invariants do not hold.
    #[inline]
    pub fn value<T>(&self) -> &'a T
    where
        T: zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
    {
        let (val, _) = T::ref_from_prefix(self.data)
            .expect("dispatch response data does not satisfy T's layout");
        val
    }
}

/// Builder for dispatching a single CMIF request in domain mode.
///
/// Wraps a [`Dispatch`] together with a [`DomainRef`] onto the parent domain.
/// The borrow lets [`send`](Self::send) construct [`DomainObject<'d>`]
/// instances from the server-emitted object ids in the response, so the
/// caller never has to launder raw `u32` ids back through an unchecked
/// constructor.
#[derive(Debug)]
pub struct DomainDispatch<'d> {
    inner: Dispatch<'d>,
    domain: DomainRef<'d>,
}

impl<'d> DomainDispatch<'d> {
    /// Creates a new domain-dispatch builder. Used by the typed wrappers.
    ///
    /// The session handle and pointer-buffer size come from `domain`, which is
    /// the only thing that can supply a matched pair of them.
    #[inline]
    pub(crate) fn new(domain: DomainRef<'d>, object_id: Option<ObjectId>, request_id: u32) -> Self {
        Self {
            inner: Dispatch::new(
                domain.handle(),
                domain.pointer_buffer_size(),
                object_id,
                request_id,
            ),
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
    /// [`DomainObject<'d>`] instances bound to the originating domain.
    /// Each server-emitted [`ObjectId`] becomes exactly one [`DomainObject`].
    ///
    /// # Panics
    ///
    /// Panics in debug builds if a buffer registered via
    /// [`inout_buffer`](Self::inout_buffer) carries
    /// [`BufferAttr::HIPC_POINTER`]; the combination is unsupported and the
    /// slot is skipped in release builds.
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
            data: resp.payload,
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
/// The lifetime `'d` ties each `DomainObject` to the parent domain.
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
    /// Borrows the response payload as a typed reference.
    ///
    /// Infallible by construction: the caller of
    /// [`DomainDispatch::out_size`] declares `size_of::<T>()`, and the
    /// underlying CMIF parser yields a 16-byte-aligned `data` slice -
    /// sufficient for any `T` whose `align_of` is ≤ 16. Panics only if
    /// those invariants do not hold.
    #[inline]
    pub fn value<T>(&self) -> &'b T
    where
        T: zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
    {
        let (val, _) = T::ref_from_prefix(self.data)
            .expect("dispatch response data does not satisfy T's layout");
        val
    }

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
