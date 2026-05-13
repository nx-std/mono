//! Fluent builder for dispatching CMIF requests.
//!
//! [`Dispatch`] is parameterized on the three primitives a request actually
//! needs: the session handle, the server's pointer-buffer size, and an
//! optional domain object id. The four typed wrappers ([`Session`],
//! [`Domain`], [`DomainObject`], [`OverrideService`]) each produce a
//! `Dispatch` via their inherent `dispatch(...)` method.
//!
//! [`Session`]: super::Session
//! [`Domain`]: super::Domain
//! [`DomainObject`]: super::DomainObject
//! [`OverrideService`]: super::OverrideService

use core::ptr;

use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::{
    cmif::{self, ObjectId},
    hipc,
};

/// Maximum number of buffers in a single dispatch.
pub const MAX_BUFFERS: usize = 8;

/// Maximum number of input objects in a single dispatch.
pub const MAX_IN_OBJECTS: usize = 8;

/// Maximum number of input handles in a single dispatch.
pub const MAX_IN_HANDLES: usize = 8;

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

    /// Sets the input data for the request.
    ///
    /// # Safety
    ///
    /// `data` must point to at least `size` readable bytes, and the pointed-to
    /// memory must remain valid until [`send`](Self::send) returns.
    #[inline]
    pub unsafe fn in_raw(mut self, data: *const u8, size: usize) -> Self {
        self.in_data = data;
        self.in_data_size = size;
        self
    }

    /// Sets the expected output data size.
    #[inline]
    pub fn out_size(mut self, size: usize) -> Self {
        self.out_data_size = size;
        self
    }

    /// Adds a buffer with the specified attributes. Silently ignored once
    /// [`MAX_BUFFERS`] slots are full.
    #[inline]
    pub fn buffer(mut self, ptr: *const u8, size: usize, attr: BufferAttr) -> Self {
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

    /// Sets the number of output objects expected.
    #[inline]
    pub fn out_objects(mut self, count: usize) -> Self {
        self.out_object_count = count;
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
    /// The returned [`DispatchResult`] borrows from the per-thread IPC TLS
    /// buffer; it is valid until the next IPC call on this thread.
    pub fn send(self) -> Result<DispatchResult<'static>, DispatchError> {
        let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

        let is_domain = self.object_id.is_some();

        let (
            num_in_auto,
            num_out_auto,
            num_in_buffers,
            num_out_buffers,
            num_inout_buffers,
            num_in_pointers,
            num_out_pointers,
            num_out_fixed_pointers,
        ) = count_buffer_kinds(&self.buffer_attrs[..self.buffer_count]);

        let mut builder = cmif::RequestFormatBuilder::new(self.request_id)
            .context(self.context)
            .data_size(self.in_data_size)
            .server_pointer_size(self.pointer_buffer_size as usize)
            .in_auto_buffers(num_in_auto)
            .out_auto_buffers(num_out_auto)
            .in_buffers(num_in_buffers)
            .out_buffers(num_out_buffers)
            .inout_buffers(num_inout_buffers)
            .in_pointers(num_in_pointers)
            .out_pointers(num_out_pointers)
            .out_fixed_pointers(num_out_fixed_pointers)
            .objects(self.in_object_count as u32)
            .handles(self.in_handle_count as u32);

        if let Some(object_id) = self.object_id {
            builder = builder.object_id(object_id);
        }

        if self.send_pid {
            builder = builder.send_pid();
        }

        let fmt = builder.build();

        // SAFETY: ipc_buf points to the current thread's IPC buffer.
        let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

        if !self.in_data.is_null() && self.in_data_size > 0 {
            // SAFETY: Caller of `in_raw` guarantees `in_data` is valid for
            // `in_data_size` bytes; `req.data` is a freshly-allocated CMIF
            // payload slot of at least that size.
            unsafe {
                ptr::copy_nonoverlapping(self.in_data, req.data.as_mut_ptr(), self.in_data_size);
            }
        }

        for i in 0..self.buffer_count {
            let buf = &self.buffers[i];
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
                    req.add_in_auto_buffer(buf.ptr, buf.size, mode);
                }
                if is_out {
                    req.add_out_auto_buffer(buf.ptr as *mut u8, buf.size, mode);
                }
            } else if attr.contains(BufferAttr::HIPC_MAP_ALIAS) {
                if is_in && is_out {
                    req.add_inout_buffer(buf.ptr as *mut u8, buf.size, mode);
                } else if is_in {
                    req.add_in_buffer(buf.ptr, buf.size, mode);
                } else if is_out {
                    req.add_out_buffer(buf.ptr as *mut u8, buf.size, mode);
                }
            } else if attr.contains(BufferAttr::HIPC_POINTER) {
                if is_in {
                    req.add_in_pointer(buf.ptr, buf.size);
                } else if is_out {
                    if attr.contains(BufferAttr::FIXED_SIZE) {
                        req.add_out_fixed_pointer(buf.ptr as *mut u8, buf.size);
                    } else {
                        req.add_out_pointer(buf.ptr as *mut u8, buf.size);
                    }
                }
            }
        }

        for i in 0..self.in_object_count {
            if let Some(obj) = self.in_objects[i] {
                req.add_object(obj);
            }
        }

        for i in 0..self.in_handle_count {
            req.add_handle(self.in_handles[i]);
        }

        ipc::send_sync_request(self.session).map_err(DispatchError::SendRequest)?;

        // SAFETY: Response is in the TLS buffer after a successful send.
        let resp = unsafe { cmif::parse_response(ipc_buf, is_domain, self.out_data_size) }
            .map_err(DispatchError::ParseResponse)?;

        Ok(DispatchResult {
            data: resp.data,
            objects: resp.objects,
            copy_handles: resp.copy_handles,
            move_handles: resp.move_handles,
        })
    }
}

/// Error returned by [`Dispatch::send`].
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    /// The kernel rejected the underlying `SendSyncRequest`.
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// The response header did not pass CMIF validation.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Result of a successful dispatch operation.
#[derive(Debug)]
pub struct DispatchResult<'a> {
    /// Response payload data.
    pub data: &'a [u8],
    /// Returned domain object IDs (domain mode only).
    pub objects: &'a [u32],
    /// Returned copy handles.
    pub copy_handles: &'a [u32],
    /// Returned move handles.
    pub move_handles: &'a [u32],
}

/// Tallies the eight CMIF buffer-kind counts from a slice of attribute flags.
fn count_buffer_kinds(attrs: &[BufferAttr]) -> (u32, u32, u32, u32, u32, u32, u32, u32) {
    let mut num_in_auto = 0u32;
    let mut num_out_auto = 0u32;
    let mut num_in_buffers = 0u32;
    let mut num_out_buffers = 0u32;
    let mut num_inout_buffers = 0u32;
    let mut num_in_pointers = 0u32;
    let mut num_out_pointers = 0u32;
    let mut num_out_fixed_pointers = 0u32;

    for attr in attrs {
        let attr = *attr;
        let is_in = attr.contains(BufferAttr::IN);
        let is_out = attr.contains(BufferAttr::OUT);

        if attr.contains(BufferAttr::HIPC_AUTO_SELECT) {
            if is_in {
                num_in_auto += 1;
            }
            if is_out {
                num_out_auto += 1;
            }
        } else if attr.contains(BufferAttr::HIPC_MAP_ALIAS) {
            if is_in && is_out {
                num_inout_buffers += 1;
            } else if is_in {
                num_in_buffers += 1;
            } else if is_out {
                num_out_buffers += 1;
            }
        } else if attr.contains(BufferAttr::HIPC_POINTER) {
            if is_in {
                num_in_pointers += 1;
            } else if is_out {
                if attr.contains(BufferAttr::FIXED_SIZE) {
                    num_out_fixed_pointers += 1;
                } else {
                    num_out_pointers += 1;
                }
            }
        }
    }

    (
        num_in_auto,
        num_out_auto,
        num_in_buffers,
        num_out_buffers,
        num_inout_buffers,
        num_in_pointers,
        num_out_pointers,
        num_out_fixed_pointers,
    )
}
