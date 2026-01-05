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

use static_assertions::const_assert_eq;

use crate::cmif;

/// Invalid handle sentinel value.
pub const INVALID_HANDLE: u32 = 0;

// Control request IDs for CMIF session management.
const CTRL_CONVERT_TO_DOMAIN: u32 = 0;
const CTRL_CLONE_OBJECT: u32 = 2;
const CTRL_QUERY_POINTER_BUFFER_SIZE: u32 = 3;
const CTRL_CLONE_OBJECT_EX: u32 = 4;

/// Queries the server's pointer buffer size via control request 3.
///
/// # Safety
///
/// `session` must be a valid IPC session handle.
pub unsafe fn query_pointer_buffer_size(session: u32) -> Result<u16, u32> {
    let tls = nx_sys_thread_tls::get_ptr();

    // SAFETY: TLS pointer is valid for the current thread.
    let ipc_buf = unsafe { (*tls).ipc_buffer.as_mut_ptr() };

    // SAFETY: ipc_buf points to valid IPC buffer with sufficient space.
    unsafe { cmif::make_control_request(ipc_buf, CTRL_QUERY_POINTER_BUFFER_SIZE, 0) };

    // SAFETY: session is a valid handle per caller contract.
    let rc = unsafe { nx_svc::raw::send_sync_request(session) };
    if rc != 0 {
        return Err(rc);
    }

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u16>() as u32) }?;

    // SAFETY: Response data contains u16 per CMIF protocol.
    let size = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u16>()) };
    Ok(size)
}

/// Clones the current session object via control request 2.
///
/// # Safety
///
/// `session` must be a valid IPC session handle.
pub unsafe fn clone_current_object(session: u32) -> Result<u32, u32> {
    let tls = nx_sys_thread_tls::get_ptr();

    // SAFETY: TLS pointer is valid for the current thread.
    let ipc_buf = unsafe { (*tls).ipc_buffer.as_mut_ptr() };

    // SAFETY: ipc_buf points to valid IPC buffer with sufficient space.
    unsafe { cmif::make_control_request(ipc_buf, CTRL_CLONE_OBJECT, 0) };

    // SAFETY: session is a valid handle per caller contract.
    let rc = unsafe { nx_svc::raw::send_sync_request(session) };
    if rc != 0 {
        return Err(rc);
    }

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }?;

    // Clone returns a move handle
    if resp.move_handles.is_empty() {
        return Err(0xFFFF);
    }
    Ok(resp.move_handles[0])
}

/// Clones the current session object with a tag via control request 4.
///
/// # Safety
///
/// `session` must be a valid IPC session handle.
pub unsafe fn clone_current_object_ex(session: u32, tag: u32) -> Result<u32, u32> {
    let tls = nx_sys_thread_tls::get_ptr();

    // SAFETY: TLS pointer is valid for the current thread.
    let ipc_buf = unsafe { (*tls).ipc_buffer.as_mut_ptr() };

    // SAFETY: ipc_buf points to valid IPC buffer with sufficient space.
    let data_ptr = unsafe {
        cmif::make_control_request(ipc_buf, CTRL_CLONE_OBJECT_EX, size_of::<u32>() as u32)
    };

    // SAFETY: data_ptr points to valid payload area within IPC buffer.
    unsafe { ptr::write_unaligned(data_ptr.cast::<u32>(), tag) };

    // SAFETY: session is a valid handle per caller contract.
    let rc = unsafe { nx_svc::raw::send_sync_request(session) };
    if rc != 0 {
        return Err(rc);
    }

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }?;

    // Clone returns a move handle
    if resp.move_handles.is_empty() {
        return Err(0xFFFF);
    }
    Ok(resp.move_handles[0])
}

/// Converts the current session to a domain via control request 0.
///
/// # Safety
///
/// `session` must be a valid IPC session handle.
pub unsafe fn convert_current_object_to_domain(session: u32) -> Result<u32, u32> {
    let tls = nx_sys_thread_tls::get_ptr();

    // SAFETY: TLS pointer is valid for the current thread.
    let ipc_buf = unsafe { (*tls).ipc_buffer.as_mut_ptr() };

    // SAFETY: ipc_buf points to valid IPC buffer with sufficient space.
    unsafe { cmif::make_control_request(ipc_buf, CTRL_CONVERT_TO_DOMAIN, 0) };

    // SAFETY: session is a valid handle per caller contract.
    let rc = unsafe { nx_svc::raw::send_sync_request(session) };
    if rc != 0 {
        return Err(rc);
    }

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u32>() as u32) }?;

    // SAFETY: Response data contains object_id as u32.
    let object_id = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };
    Ok(object_id)
}

/// IPC service wrapper.
///
/// Wraps a session handle with metadata for domain support and pointer buffer
/// tracking. The struct layout matches libnx's `Service` exactly for FFI.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Service {
    /// IPC session handle.
    pub session: u32,
    /// Whether this service owns the session handle (1 = yes, 0 = no).
    pub own_handle: u32,
    /// Domain object ID (0 = non-domain or override).
    pub object_id: u32,
    /// Server's pointer buffer size for auto-select buffers.
    pub pointer_buffer_size: u16,
}
const_assert_eq!(size_of::<Service>(), 16);

impl Default for Service {
    fn default() -> Self {
        Self {
            session: INVALID_HANDLE,
            own_handle: 0,
            object_id: 0,
            pointer_buffer_size: 0,
        }
    }
}

impl Service {
    /// Creates a new service from a session handle.
    ///
    /// Queries the server's pointer buffer size automatically.
    ///
    /// # Safety
    ///
    /// `handle` must be a valid IPC session handle.
    pub unsafe fn new(handle: u32) -> Result<Self, u32> {
        // SAFETY: handle is valid per caller contract.
        let pointer_buffer_size = unsafe { query_pointer_buffer_size(handle) }.unwrap_or(0);

        Ok(Self {
            session: handle,
            own_handle: 1,
            object_id: 0,
            pointer_buffer_size,
        })
    }

    /// Creates a non-domain subservice from a parent service's handle.
    ///
    /// The new service inherits the parent's pointer buffer size but owns
    /// the provided handle independently.
    pub fn new_subservice(parent: &Service, handle: u32) -> Self {
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
    pub fn new_domain_subservice(parent: &Service, object_id: u32) -> Self {
        Self {
            session: parent.session,
            own_handle: 0,
            object_id,
            pointer_buffer_size: parent.pointer_buffer_size,
        }
    }

    /// Closes the service and releases resources.
    ///
    /// For domain subservices, sends a domain close request. For services
    /// that own their handle, closes the kernel handle.
    pub fn close(&mut self) {
        if !self.is_active() {
            return;
        }

        let tls = nx_sys_thread_tls::get_ptr();
        // SAFETY: TLS pointer is valid for the current thread.
        let ipc_buf = unsafe { (*tls).ipc_buffer.as_mut_ptr() };

        // Determine what to close based on ownership
        let close_object_id = if self.own_handle != 0 {
            0
        } else {
            self.object_id
        };

        // SAFETY: ipc_buf points to valid IPC buffer.
        unsafe { cmif::make_close_request(ipc_buf, close_object_id) };

        // SAFETY: session is valid for active service.
        let _ = unsafe { nx_svc::raw::send_sync_request(self.session) };

        // Close the handle if we own it
        if self.own_handle != 0 {
            // SAFETY: session handle is valid and owned.
            let _ = unsafe { nx_svc::raw::close_handle(self.session) };
        }

        // Reset to default state
        *self = Self::default();
    }

    /// Clones the current service.
    ///
    /// Returns a new service with a cloned session handle.
    pub fn try_clone(&self) -> Result<Service, u32> {
        if !self.is_active() {
            return Err(0xFFFF);
        }

        // SAFETY: session is valid for active service.
        let new_handle = unsafe { clone_current_object(self.session) }?;

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
    pub fn try_clone_ex(&self, tag: u32) -> Result<Service, u32> {
        if !self.is_active() {
            return Err(0xFFFF);
        }

        // SAFETY: session is valid for active service.
        let new_handle = unsafe { clone_current_object_ex(self.session, tag) }?;

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
    pub fn convert_to_domain(&mut self) -> Result<(), u32> {
        if !self.is_active() {
            return Err(0xFFFF);
        }

        // SAFETY: session is valid for active service.
        let object_id = unsafe { convert_current_object_to_domain(self.session) }?;

        self.object_id = object_id;
        Ok(())
    }

    /// Returns whether the service has a valid session handle.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.session != INVALID_HANDLE
    }

    /// Returns whether this is an override service (Rust implementation).
    ///
    /// Override services have a valid session but don't own the handle and
    /// have no domain object ID.
    #[inline]
    pub fn is_override(&self) -> bool {
        self.is_active() && self.own_handle == 0 && self.object_id == 0
    }

    /// Returns whether this is a domain service (owns handle with object ID).
    #[inline]
    pub fn is_domain(&self) -> bool {
        self.is_active() && self.own_handle != 0 && self.object_id != 0
    }

    /// Returns whether this is a domain subservice (shares handle).
    #[inline]
    pub fn is_domain_subservice(&self) -> bool {
        self.is_active() && self.own_handle == 0 && self.object_id != 0
    }

    /// Creates a dispatch builder for sending a command to this service.
    #[inline]
    pub fn dispatch(&self, request_id: u32) -> Dispatch<'_> {
        Dispatch::new(self, request_id)
    }
}

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
    in_objects: [u32; MAX_IN_OBJECTS],
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
            in_objects: [0; MAX_IN_OBJECTS],
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
    pub fn in_object(mut self, object_id: u32) -> Self {
        if self.in_object_count < MAX_IN_OBJECTS {
            self.in_objects[self.in_object_count] = object_id;
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
    pub fn send(self) -> Result<DispatchResult<'static>, u32> {
        if !self.service.is_active() {
            return Err(0xFFFF);
        }

        let tls = nx_sys_thread_tls::get_ptr();
        // SAFETY: TLS pointer is valid for the current thread.
        let ipc_buf = unsafe { (*tls).ipc_buffer.as_mut_ptr() };

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

        let fmt = cmif::RequestFormat {
            object_id: if is_domain { self.service.object_id } else { 0 },
            request_id: self.request_id,
            context: self.context,
            data_size: self.in_data_size as u32,
            server_pointer_size: self.service.pointer_buffer_size as u32,
            num_in_auto_buffers: num_in_auto,
            num_out_auto_buffers: num_out_auto,
            num_in_buffers,
            num_out_buffers,
            num_inout_buffers,
            num_in_pointers,
            num_out_pointers,
            num_out_fixed_pointers,
            num_objects: self.in_object_count as u32,
            num_handles: self.in_handle_count as u32,
            send_pid: self.send_pid,
        };

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
            req.add_object(self.in_objects[i]);
        }

        // Add input handles
        for i in 0..self.in_handle_count {
            req.add_handle(self.in_handles[i]);
        }

        // Send the request
        // SAFETY: session is valid for active service.
        let rc = unsafe { nx_svc::raw::send_sync_request(self.service.session) };
        if rc != 0 {
            return Err(rc);
        }

        // Parse response
        // SAFETY: Response is in TLS buffer after successful send.
        let resp = unsafe { cmif::parse_response(ipc_buf, is_domain, self.out_data_size as u32) }?;

        Ok(DispatchResult {
            data: resp.data,
            objects: resp.objects,
            copy_handles: resp.copy_handles,
            move_handles: resp.move_handles,
        })
    }
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
