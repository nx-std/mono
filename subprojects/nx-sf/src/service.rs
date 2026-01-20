//! Service abstraction for IPC communication.
//!
//! A [`Service`] wraps an IPC session handle and provides methods for
//! dispatching CMIF/TIPC requests to Horizon OS services.
//!
//! # Service Types
//!
//! Services can be in different states based on their field values:
//!
//! | Type | own_handle | object_id | Description |
//! |------|------------|-----------|-------------|
//! | Override | 0 | 0 | Rust implementation taking over |
//! | Non-Domain | 1 | 0 | Standard session handle |
//! | Domain | 1 | != 0 | Domain root (owns the handle) |
//! | Domain Subservice | 0 | != 0 | Domain child (shares handle) |
//!
//! # References
//!
//! - libnx `sf/service.h`

use core::{mem::size_of, ptr};

use nx_svc::ipc::{self, Handle as SessionHandle};
use static_assertions::const_assert_eq;

use crate::cmif::{self, ObjectId};

// Control request IDs for CMIF session management.
const CTRL_CONVERT_TO_DOMAIN: u32 = 0;
const CTRL_COPY_FROM_DOMAIN: u32 = 1;
const CTRL_CLONE_OBJECT: u32 = 2;
const CTRL_QUERY_POINTER_BUFFER_SIZE: u32 = 3;
const CTRL_CLONE_OBJECT_EX: u32 = 4;

/// Maximum number of buffers in a single dispatch.
pub const MAX_BUFFERS: usize = 8;

/// Maximum number of input objects in a single dispatch.
pub const MAX_IN_OBJECTS: usize = 8;

/// Maximum number of input handles in a single dispatch.
pub const MAX_IN_HANDLES: usize = 8;

/// IPC service wrapper.
///
/// Wraps a session handle with metadata for domain support and pointer buffer
/// tracking. The struct layout matches libnx's `Service` exactly for FFI.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Service {
    /// IPC session handle.
    pub session: SessionHandle,
    /// Whether this service owns the session handle (1 = yes, 0 = no).
    pub own_handle: u32,
    /// Domain object ID (0 = non-domain or override).
    pub object_id: u32,
    /// Server's pointer buffer size for auto-select buffers.
    pub pointer_buffer_size: u16,
}
const_assert_eq!(size_of::<Service>(), 16);

impl Service {
    /// Creates a new service from a session handle.
    ///
    /// Queries the server's pointer buffer size automatically.
    /// If the query fails, pointer buffer size defaults to 0.
    pub fn new(handle: SessionHandle) -> Self {
        let pointer_buffer_size = query_pointer_buffer_size(handle).unwrap_or(0);

        Self {
            session: handle,
            own_handle: 1,
            object_id: 0,
            pointer_buffer_size,
        }
    }

    /// Creates a non-domain subservice from a parent service's handle.
    ///
    /// The new service inherits the parent's pointer buffer size but owns
    /// the provided handle independently.
    pub fn new_subservice(parent: &Service, handle: SessionHandle) -> Self {
        Self {
            session: handle,
            own_handle: 1,
            object_id: 0,
            pointer_buffer_size: parent.pointer_buffer_size,
        }
    }

    /// Creates a domain subservice from a parent domain service.
    ///
    /// The new service shares the parent's session handle but has its own
    /// domain object ID.
    pub fn new_domain_subservice(parent: &Service, object_id: ObjectId) -> Self {
        Self {
            session: parent.session,
            own_handle: 0,
            object_id: object_id.to_raw(),
            pointer_buffer_size: parent.pointer_buffer_size,
        }
    }

    /// Closes the service and releases resources.
    ///
    /// Consumes `self` to prevent use-after-close.
    pub fn close(self) {
        let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

        // Determine what to close based on ownership.
        // If we own the handle, close the session (None).
        // Otherwise, close the domain object (Some(object_id)).
        let close_object_id = if self.own_handle != 0 {
            None
        } else {
            ObjectId::new(self.object_id)
        };

        // SAFETY: ipc_buf points to valid IPC buffer.
        unsafe { cmif::make_close_request(ipc_buf, close_object_id) };

        // Send close request (ignore errors)
        let _ = ipc::send_sync_request(self.session);

        // Close the handle if we own it
        if self.own_handle != 0 {
            let _ = ipc::close_handle(self.session);
        }
    }

    /// Clones the current service.
    ///
    /// Returns a new service with a cloned session handle.
    pub fn try_clone(&self) -> Result<Service, TryCloneError> {
        let new_handle = clone_current_object(self.session).map_err(TryCloneError)?;

        Ok(Self {
            session: new_handle,
            own_handle: 1,
            object_id: 0,
            pointer_buffer_size: self.pointer_buffer_size,
        })
    }

    /// Clones the current service with a tag.
    ///
    /// Returns a new service with a cloned session handle.
    pub fn try_clone_ex(&self, tag: u32) -> Result<Service, TryCloneExError> {
        let new_handle = clone_current_object_ex(self.session, tag).map_err(TryCloneExError)?;

        Ok(Self {
            session: new_handle,
            own_handle: 1,
            object_id: 0,
            pointer_buffer_size: self.pointer_buffer_size,
        })
    }

    /// Converts the service to a domain.
    ///
    /// After conversion, the service can multiplex multiple objects over
    /// a single session handle.
    pub fn convert_to_domain(&mut self) -> Result<(), ServiceConvertToDomainError> {
        let object_id =
            convert_current_object_to_domain(self.session).map_err(ServiceConvertToDomainError)?;
        self.object_id = object_id.to_raw();
        Ok(())
    }

    /// Copies a domain object to a new standalone session handle.
    ///
    /// This extracts the specified domain object as an independent service
    /// with its own session handle. Only valid for domain services.
    pub fn copy_object_to_session(
        &self,
        object_id: ObjectId,
    ) -> Result<Service, CopyObjectToSessionError> {
        if !self.is_domain() && !self.is_domain_subservice() {
            return Err(CopyObjectToSessionError::NotDomain);
        }

        let new_handle = copy_from_current_domain(self.session, object_id)
            .map_err(CopyObjectToSessionError::CopyFailed)?;

        Ok(Self {
            session: new_handle,
            own_handle: 1,
            object_id: 0,
            pointer_buffer_size: self.pointer_buffer_size,
        })
    }

    /// Returns whether this is an override service (Rust implementation).
    ///
    /// Override services don't own the handle and have no domain object ID.
    #[inline]
    pub fn is_override(&self) -> bool {
        self.own_handle == 0 && self.object_id == 0
    }

    /// Returns whether this is a domain service (owns handle with object ID).
    #[inline]
    pub fn is_domain(&self) -> bool {
        self.own_handle != 0 && self.object_id != 0
    }

    /// Returns whether this is a domain subservice (shares handle).
    #[inline]
    pub fn is_domain_subservice(&self) -> bool {
        self.own_handle == 0 && self.object_id != 0
    }

    /// Creates a dispatch builder for sending a command to this service.
    #[inline]
    pub fn dispatch(&self, request_id: u32) -> Dispatch<'_> {
        Dispatch::new(self, request_id)
    }
}

/// Error returned by [`Service::try_clone`].
#[derive(Debug, thiserror::Error)]
#[error("failed to clone service")]
pub struct TryCloneError(#[source] pub CloneObjectError);

/// Error returned by [`Service::try_clone_ex`].
#[derive(Debug, thiserror::Error)]
#[error("failed to clone service with tag")]
pub struct TryCloneExError(#[source] pub CloneObjectExError);

/// Error returned by [`Service::convert_to_domain`].
#[derive(Debug, thiserror::Error)]
#[error("failed to convert service to domain")]
pub struct ServiceConvertToDomainError(#[source] pub ConvertToDomainError);

/// Error returned by [`Service::copy_object_to_session`].
#[derive(Debug, thiserror::Error)]
pub enum CopyObjectToSessionError {
    /// Service is not a domain or domain subservice.
    #[error("service is not a domain")]
    NotDomain,
    /// Failed to copy the domain object.
    #[error("failed to copy domain object")]
    CopyFailed(#[source] CopyFromDomainError),
}

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

    /// Checks if flag is set.
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

/// Builder for dispatching CMIF commands to a service.
#[derive(Debug)]
pub struct Dispatch<'a> {
    service: &'a Service,
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
    /// Creates a new dispatch builder for the given service and request ID.
    fn new(service: &'a Service, request_id: u32) -> Self {
        Self {
            service,
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
    /// The data pointer must remain valid until `send()` is called.
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

    /// Adds a buffer with the specified attributes.
    #[inline]
    pub fn buffer(mut self, ptr: *const u8, size: usize, attr: BufferAttr) -> Self {
        if self.buffer_count < MAX_BUFFERS {
            self.buffers[self.buffer_count] = Buffer { ptr, size };
            self.buffer_attrs[self.buffer_count] = attr;
            self.buffer_count += 1;
        }
        self
    }

    /// Adds an input domain object.
    #[inline]
    pub fn in_object(mut self, object_id: ObjectId) -> Self {
        if self.in_object_count < MAX_IN_OBJECTS {
            self.in_objects[self.in_object_count] = Some(object_id);
            self.in_object_count += 1;
        }
        self
    }

    /// Adds an input handle.
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

    /// Sets an output handle attribute at the given index.
    #[inline]
    pub fn out_handle(mut self, index: usize, attr: OutHandleAttr) -> Self {
        if index < MAX_BUFFERS {
            self.out_handle_attrs[index] = attr;
        }
        self
    }

    /// Enables sending the process ID.
    #[inline]
    pub fn send_pid(mut self) -> Self {
        self.send_pid = true;
        self
    }

    /// Sends the dispatch request and returns the result.
    ///
    /// On success, returns a [`DispatchResult`] containing response data,
    /// handles, and objects. The returned data references the TLS IPC buffer
    /// and is valid until the next IPC call on this thread.
    pub fn send(self) -> Result<DispatchResult<'static>, DispatchError> {
        let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

        let is_domain = self.service.is_domain() || self.service.is_domain_subservice();

        // Count buffer types for CMIF format
        let mut num_in_auto = 0u32;
        let mut num_out_auto = 0u32;
        let mut num_in_buffers = 0u32;
        let mut num_out_buffers = 0u32;
        let mut num_inout_buffers = 0u32;
        let mut num_in_pointers = 0u32;
        let mut num_out_pointers = 0u32;
        let mut num_out_fixed_pointers = 0u32;

        for i in 0..self.buffer_count {
            let attr = self.buffer_attrs[i];
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

        let mut builder = cmif::RequestFormatBuilder::new(self.request_id)
            .context(self.context)
            .data_size(self.in_data_size)
            .server_pointer_size(self.service.pointer_buffer_size as usize)
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

        if is_domain && let Some(object_id) = ObjectId::new(self.service.object_id) {
            builder = builder.object_id(object_id);
        }

        if self.send_pid {
            builder = builder.send_pid();
        }

        let fmt = builder.build();

        // SAFETY: ipc_buf points to valid IPC buffer.
        let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

        // Copy input data
        if !self.in_data.is_null() && self.in_data_size > 0 {
            // SAFETY: in_data and req.data are valid, sizes match.
            unsafe {
                ptr::copy_nonoverlapping(self.in_data, req.data.as_mut_ptr(), self.in_data_size);
            }
        }

        // Add buffers
        for i in 0..self.buffer_count {
            let buf = &self.buffers[i];
            let attr = self.buffer_attrs[i];
            let is_in = attr.contains(BufferAttr::IN);
            let is_out = attr.contains(BufferAttr::OUT);

            // Determine buffer mode
            let mode = if attr.contains(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE) {
                crate::hipc::BufferMode::NonSecure
            } else if attr.contains(BufferAttr::MAP_TRANSFER_ALLOWS_NON_DEVICE) {
                crate::hipc::BufferMode::NonDevice
            } else {
                crate::hipc::BufferMode::Normal
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

        // Add input objects (domain mode)
        for i in 0..self.in_object_count {
            if let Some(obj) = self.in_objects[i] {
                req.add_object(obj);
            }
        }

        // Add input handles
        for i in 0..self.in_handle_count {
            req.add_handle(self.in_handles[i]);
        }

        // Send the request
        ipc::send_sync_request(self.service.session).map_err(DispatchError::SendRequest)?;

        // Parse response
        // SAFETY: Response is in TLS buffer after successful send.
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
    /// Failed to send the IPC request.
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the service response.
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

// =============================================================================
// Helper functions for CMIF control requests
// =============================================================================

/// Queries the server's pointer buffer size via control request 3.
pub fn query_pointer_buffer_size(
    session: SessionHandle,
) -> Result<u16, QueryPointerBufferSizeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // SAFETY: ipc_buf points to valid IPC buffer with sufficient space.
    unsafe { cmif::make_control_request(ipc_buf, CTRL_QUERY_POINTER_BUFFER_SIZE, 0) };

    ipc::send_sync_request(session).map_err(QueryPointerBufferSizeError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u16>()) }
        .map_err(QueryPointerBufferSizeError::ParseResponse)?;

    // SAFETY: Response data contains u16 per CMIF protocol.
    let size = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u16>()) };
    Ok(size)
}

/// Error returned by [`query_pointer_buffer_size`].
#[derive(Debug, thiserror::Error)]
pub enum QueryPointerBufferSizeError {
    /// Failed to send the IPC request.
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the service response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Clones the current session object via control request 2.
pub fn clone_current_object(session: SessionHandle) -> Result<SessionHandle, CloneObjectError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // SAFETY: ipc_buf points to valid IPC buffer with sufficient space.
    unsafe { cmif::make_control_request(ipc_buf, CTRL_CLONE_OBJECT, 0) };

    ipc::send_sync_request(session).map_err(CloneObjectError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(CloneObjectError::ParseResponse)?;

    // Clone returns a move handle
    if resp.move_handles.is_empty() {
        return Err(CloneObjectError::MissingHandle);
    }

    // SAFETY: Kernel returned a valid handle in the response.
    Ok(unsafe { SessionHandle::from_raw(resp.move_handles[0]) })
}

/// Error returned by [`clone_current_object`].
#[derive(Debug, thiserror::Error)]
pub enum CloneObjectError {
    /// Failed to send the IPC request.
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the service response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Response did not contain the expected move handle.
    #[error("missing move handle in response")]
    MissingHandle,
}

/// Copies a domain object to a new session handle via control request 1.
///
/// This extracts a domain object as a standalone session handle, allowing
/// it to be used independently of the domain.
pub fn copy_from_current_domain(
    session: SessionHandle,
    object_id: ObjectId,
) -> Result<SessionHandle, CopyFromDomainError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // SAFETY: ipc_buf points to valid IPC buffer with sufficient space.
    let data_ptr = unsafe {
        cmif::make_control_request(ipc_buf, CTRL_COPY_FROM_DOMAIN, size_of::<u32>() as u32)
    };

    // SAFETY: data_ptr points to valid payload area within IPC buffer.
    unsafe { ptr::write_unaligned(data_ptr.cast::<u32>(), object_id.to_raw()) };

    ipc::send_sync_request(session).map_err(CopyFromDomainError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(CopyFromDomainError::ParseResponse)?;

    if resp.move_handles.is_empty() {
        return Err(CopyFromDomainError::MissingHandle);
    }

    // SAFETY: Kernel returned a valid handle in the response.
    Ok(unsafe { SessionHandle::from_raw(resp.move_handles[0]) })
}

/// Error returned by [`copy_from_current_domain`].
#[derive(Debug, thiserror::Error)]
pub enum CopyFromDomainError {
    /// Failed to send the IPC request.
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the service response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Response did not contain the expected move handle.
    #[error("missing move handle in response")]
    MissingHandle,
}

/// Clones the current session object with a tag via control request 4.
pub fn clone_current_object_ex(
    session: SessionHandle,
    tag: u32,
) -> Result<SessionHandle, CloneObjectExError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // SAFETY: ipc_buf points to valid IPC buffer with sufficient space.
    let data_ptr = unsafe {
        cmif::make_control_request(ipc_buf, CTRL_CLONE_OBJECT_EX, size_of::<u32>() as u32)
    };

    // SAFETY: data_ptr points to valid payload area within IPC buffer.
    unsafe { ptr::write_unaligned(data_ptr.cast::<u32>(), tag) };

    ipc::send_sync_request(session).map_err(CloneObjectExError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(CloneObjectExError::ParseResponse)?;

    // Clone returns a move handle
    if resp.move_handles.is_empty() {
        return Err(CloneObjectExError::MissingHandle);
    }

    // SAFETY: Kernel returned a valid handle in the response.
    Ok(unsafe { SessionHandle::from_raw(resp.move_handles[0]) })
}

/// Error returned by [`clone_current_object_ex`].
#[derive(Debug, thiserror::Error)]
pub enum CloneObjectExError {
    /// Failed to send the IPC request.
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the service response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Response did not contain the expected move handle.
    #[error("missing move handle in response")]
    MissingHandle,
}

/// Converts the current session to a domain via control request 0.
pub fn convert_current_object_to_domain(
    session: SessionHandle,
) -> Result<ObjectId, ConvertToDomainError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // SAFETY: ipc_buf points to valid IPC buffer with sufficient space.
    unsafe { cmif::make_control_request(ipc_buf, CTRL_CONVERT_TO_DOMAIN, 0) };

    ipc::send_sync_request(session).map_err(ConvertToDomainError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u32>()) }
        .map_err(ConvertToDomainError::ParseResponse)?;

    // SAFETY: Response data contains object_id as u32. The kernel always returns
    // a valid non-zero object ID when converting to domain.
    let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };
    Ok(unsafe { ObjectId::new_unchecked(raw) })
}

/// Error returned by [`convert_current_object_to_domain`].
#[derive(Debug, thiserror::Error)]
pub enum ConvertToDomainError {
    /// Failed to send the IPC request.
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the service response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
