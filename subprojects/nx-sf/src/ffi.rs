//! FFI bindings for nx-sf service functionality.
//!
//! Provides C-compatible exports for service operations. libnx's service
//! functions are inline, so these exports primarily exist for C callers that
//! want to use the Rust implementation directly and for tests. The actual
//! link-time override of libnx happens at the SVC layer in nx-svc.
//!
//! # Layout
//!
//! The exported [`Service`] struct is byte-compatible with libnx's `Service`
//! (16 bytes; `const_assert_eq` enforces this). Safe Rust code holding a service of its own uses
//! [`Session`](crate::service::Session) / [`Domain`](crate::service::Domain)
//! / [`DomainObject`](crate::service::DomainObject) / [`OverrideService`](crate::service::OverrideService)
//! instead, and the FFI symbols translate between the two at the boundary.
//!
//! A C caller that owns one and passes it in is the other direction, and it is the reason this
//! struct is `pub`: an entry point elsewhere in the workspace that receives a `Service *` reads it
//! here and addresses it with [`Service::as_domain_object`] or [`Service::as_session`]. Both
//! borrow, because the struct and everything it names belong to the caller.
//!
//! # Naming Convention
//!
//! FFI exports follow the pattern `__nx_sf__<fn_name>` (see
//! `docs/libnx_overrides.md`).

use core::mem::{
    self,
    size_of,
};

use nx_svc::{
    ipc::Handle as SessionHandle,
    raw::INVALID_HANDLE,
};
use static_assertions::const_assert_eq;

use crate::{
    cmif::ObjectId,
    error::ToResultCode as _,
    service::{
        self,
        ForeignDomainObject,
        OverrideService,
        handle::BorrowedSessionHandle,
    },
};

/// libnx-compatible `Service` struct.
///
/// Mode is determined by the (`own_handle`, `object_id`) tuple, matching the
/// libnx encoding:
///
/// | Mode              | own_handle | object_id |
/// |-------------------|------------|-----------|
/// | Override          | 0          | 0         |
/// | Non-domain        | 1          | 0         |
/// | Domain root       | 1          | != 0      |
/// | Domain subservice | 0          | != 0      |
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Service {
    pub session: SessionHandle,
    pub own_handle: u32,
    pub object_id: u32,
    pub pointer_buffer_size: u16,
}
const_assert_eq!(size_of::<Service>(), 16);

impl Service {
    #[inline]
    fn is_override(&self) -> bool {
        self.own_handle == 0 && self.object_id == 0
    }

    #[inline]
    fn is_domain(&self) -> bool {
        self.own_handle != 0 && self.object_id != 0
    }

    #[inline]
    fn is_domain_subservice(&self) -> bool {
        self.own_handle == 0 && self.object_id != 0
    }

    /// Addresses this C-owned service as an object inside a domain.
    ///
    /// Returns `None` when the struct names no object, which is what a service the C side never
    /// converted to a domain looks like. Dispatching at that one is [`Self::as_session`]'s job.
    ///
    /// The view borrows: the C caller owns the struct, the session it names and the object inside
    /// it, and remains the sole closer of each.
    ///
    /// This is the only way to obtain a [`ForeignDomainObject`], which is why its constructor is
    /// crate-private: the proof its precondition asks for is exactly "a C caller handed us this
    /// struct", and that fact exists here and nowhere else.
    #[inline]
    pub fn as_domain_object(&self) -> Option<ForeignDomainObject<'static>> {
        // SAFETY: the C caller owns this struct and everything it names, and libnx records a
        // domain subservice by writing the parent session's handle, the size that owner queried,
        // and an id the server issued. Reading the three back addresses what it already holds;
        // nothing here closes any of them.
        ForeignDomainObject::new_unchecked(
            borrow(self.session),
            self.pointer_buffer_size,
            self.object_id,
        )
    }

    /// Addresses this C-owned service as a plain, non-domain session.
    ///
    /// The view borrows, for the reason given on [`Self::as_domain_object`].
    #[inline]
    pub fn as_session(&self) -> OverrideService {
        OverrideService::new_unchecked(self.session, self.pointer_buffer_size)
    }
}

/// Creates a service object from an IPC session handle.
///
/// # Safety
///
/// `s` must point to valid, writable memory for a Service struct.
/// `h` must be a valid IPC session handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_create(s: *mut Service, h: u32) {
    // SAFETY: h is a valid handle per caller contract.
    let handle = SessionHandle::from_raw_unchecked(h);
    let pointer_buffer_size = service::query_pointer_buffer_size(borrow(handle)).unwrap_or(0);

    // SAFETY: Caller guarantees s points to valid memory.
    unsafe {
        *s = Service {
            session: handle,
            own_handle: 1,
            object_id: 0,
            pointer_buffer_size,
        };
    }
}

/// Creates a non-domain subservice from a parent service.
///
/// # Safety
///
/// `s` and `parent` must point to valid Service structs.
/// `h` must be a valid IPC session handle (or 0 to zero-initialize).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_create_non_domain_subservice(
    s: *mut Service,
    parent: *const Service,
    h: u32,
) {
    // SAFETY: Caller guarantees pointers are valid.
    let parent = unsafe { &*parent };

    if h != INVALID_HANDLE {
        // SAFETY: h is a valid handle per caller contract.
        let handle = SessionHandle::from_raw_unchecked(h);
        // SAFETY: s points to valid memory.
        unsafe {
            *s = Service {
                session: handle,
                own_handle: 1,
                object_id: 0,
                pointer_buffer_size: parent.pointer_buffer_size,
            };
        }
    } else {
        // SAFETY: Service is repr(C) and can be zero-initialized for FFI.
        unsafe { *s = mem::zeroed() };
    }
}

/// Creates a domain subservice from a parent service.
///
/// # Safety
///
/// `s` and `parent` must point to valid Service structs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_create_domain_subservice(
    s: *mut Service,
    parent: *const Service,
    object_id: u32,
) {
    // SAFETY: Caller guarantees pointers are valid.
    let parent = unsafe { &*parent };

    if ObjectId::new(object_id).is_some() {
        // SAFETY: s points to valid memory.
        unsafe {
            *s = Service {
                session: parent.session,
                own_handle: 0,
                object_id,
                pointer_buffer_size: parent.pointer_buffer_size,
            };
        }
    } else {
        // SAFETY: Service is repr(C) and can be zero-initialized for FFI.
        unsafe { *s = mem::zeroed() };
    }
}

/// Closes a service and releases its resources.
///
/// # Safety
///
/// `s` must point to a valid Service struct. After this call, the Service at
/// `s` is zeroed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_close(s: *mut Service) {
    // SAFETY: Caller guarantees s points to valid Service.
    let srv = unsafe { *s };

    // Mirror libnx semantics: domain subservices send a per-object close on
    // the shared handle; everything else with `own_handle != 0` sends a
    // session close and releases the handle.
    if srv.own_handle != 0 {
        service::control::close_session(borrow(srv.session));
    } else if let Some(object_id) = ObjectId::new(srv.object_id) {
        service::control::close_object(borrow(srv.session), object_id);
    }

    // SAFETY: s points to valid writable memory.
    unsafe { *s = mem::zeroed() };
}

/// Clones a service.
///
/// # Safety
///
/// `s` and `out_s` must point to valid Service structs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_clone(s: *const Service, out_s: *mut Service) -> u32 {
    // SAFETY: Caller guarantees pointers are valid.
    let srv = unsafe { &*s };
    let out = unsafe { &mut *out_s };

    match service::clone_current_object(borrow(srv.session)) {
        Ok(new_handle) => {
            *out = Service {
                session: new_handle.into_handle(),
                own_handle: 1,
                object_id: 0,
                pointer_buffer_size: srv.pointer_buffer_size,
            };
            0
        }
        Err(err) => err.to_rc(),
    }
}

/// Clones a service with a session manager tag.
///
/// # Safety
///
/// `s` and `out_s` must point to valid Service structs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_clone_ex(
    s: *const Service,
    tag: u32,
    out_s: *mut Service,
) -> u32 {
    // SAFETY: Caller guarantees pointers are valid.
    let srv = unsafe { &*s };
    let out = unsafe { &mut *out_s };

    match service::clone_current_object_ex(borrow(srv.session), tag) {
        Ok(new_handle) => {
            *out = Service {
                session: new_handle.into_handle(),
                own_handle: 1,
                object_id: 0,
                pointer_buffer_size: srv.pointer_buffer_size,
            };
            0
        }
        Err(err) => err.to_rc(),
    }
}

/// Converts a service to a domain.
///
/// # Safety
///
/// `s` must point to a valid Service struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_convert_to_domain(s: *mut Service) -> u32 {
    // SAFETY: Caller guarantees s points to valid Service.
    let srv = unsafe { &mut *s };

    // For override services, clone first to obtain an owned handle, matching
    // libnx behavior.
    if srv.is_override() {
        match service::clone_current_object_ex(borrow(srv.session), 0) {
            Ok(new_handle) => {
                srv.session = new_handle.into_handle();
                srv.own_handle = 1;
            }
            Err(err) => return err.to_rc(),
        }
    }

    match service::convert_current_object_to_domain(borrow(srv.session)) {
        Ok(object_id) => {
            srv.object_id = object_id.to_raw();
            0
        }
        Err(err) => err.to_rc(),
    }
}

/// Returns whether a service is active (has valid session handle).
///
/// # Safety
///
/// `s` must be null or point to a valid Service struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_is_active(s: *const Service) -> bool {
    if s.is_null() {
        return false;
    }
    // SAFETY: Caller guarantees s is null or points to valid Service.
    unsafe { (*s).session.to_raw() != INVALID_HANDLE }
}

/// Returns whether a service is an override service.
///
/// # Safety
///
/// `s` must be null or point to a valid Service struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_is_override(s: *const Service) -> bool {
    if s.is_null() {
        return false;
    }
    // SAFETY: Caller guarantees s is null or points to valid Service.
    unsafe { (*s).is_override() }
}

/// Returns whether a service is a domain.
///
/// # Safety
///
/// `s` must be null or point to a valid Service struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_is_domain(s: *const Service) -> bool {
    if s.is_null() {
        return false;
    }
    // SAFETY: Caller guarantees s is null or points to valid Service.
    unsafe { (*s).is_domain() }
}

/// Returns whether a service is a domain subservice.
///
/// # Safety
///
/// `s` must be null or point to a valid Service struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_is_domain_subservice(s: *const Service) -> bool {
    if s.is_null() {
        return false;
    }
    // SAFETY: Caller guarantees s is null or points to valid Service.
    unsafe { (*s).is_domain_subservice() }
}

/// Returns the object ID for a domain service.
///
/// # Safety
///
/// `s` must be null or point to a valid Service struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_get_object_id(s: *const Service) -> u32 {
    if s.is_null() {
        return 0;
    }
    // SAFETY: Caller guarantees s is null or points to valid Service.
    unsafe { (*s).object_id }
}

/// Borrows the session handle held in a C-owned [`Service`] struct.
///
/// The C caller owns the `Service` and closes its handle, so every call this module makes
/// borrows for the duration of the call rather than adopting ownership. A handle the C side
/// closes while a borrow is live names a number the kernel may have reused.
#[inline]
fn borrow(handle: SessionHandle) -> BorrowedSessionHandle<'static> {
    // SAFETY: The `Service` struct outlives each individual call made through it, as the C
    // ownership contract on these entry points requires.
    BorrowedSessionHandle::from_handle_unchecked(handle)
}
