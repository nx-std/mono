//! FFI bindings for nx-sf service functionality.
//!
//! Provides C-compatible exports for service operations. Since libnx's service
//! functions are inline, these exports are primarily useful for:
//! - C code that wants to use the Rust implementation directly
//! - Testing and debugging
//!
//! The actual link-time override of libnx happens at the SVC layer (nx-svc).
//!
//! # Naming Convention
//!
//! FFI exports follow the pattern: `__nx_sf__<fn_name>`
//! See `docs/libnx_overrides.md` for details.

use crate::service::{self, INVALID_HANDLE, Service};

/// Creates a service object from an IPC session handle.
///
/// # Safety
///
/// `s` must point to valid, writable memory for a Service struct.
/// `h` must be a valid IPC session handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_create(s: *mut Service, h: u32) {
    // SAFETY: Caller guarantees s points to valid memory.
    let srv = unsafe { &mut *s };

    // Query pointer buffer size, ignoring errors
    // SAFETY: h is a valid handle per caller contract.
    let pointer_buffer_size = unsafe { service::query_pointer_buffer_size(h) }.unwrap_or(0);

    srv.session = h;
    srv.own_handle = 1;
    srv.object_id = 0;
    srv.pointer_buffer_size = pointer_buffer_size;
}

/// Creates a non-domain subservice from a parent service.
///
/// # Safety
///
/// `s` and `parent` must point to valid Service structs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_create_non_domain_subservice(
    s: *mut Service,
    parent: *const Service,
    h: u32,
) {
    // SAFETY: Caller guarantees pointers are valid.
    let srv = unsafe { &mut *s };
    let parent = unsafe { &*parent };

    if h != INVALID_HANDLE {
        srv.session = h;
        srv.own_handle = 1;
        srv.object_id = 0;
        srv.pointer_buffer_size = parent.pointer_buffer_size;
    } else {
        *srv = Service::default();
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
    let srv = unsafe { &mut *s };
    let parent = unsafe { &*parent };

    if object_id != 0 {
        srv.session = parent.session;
        srv.own_handle = 0;
        srv.object_id = object_id;
        srv.pointer_buffer_size = parent.pointer_buffer_size;
    } else {
        *srv = Service::default();
    }
}

/// Closes a service and releases its resources.
///
/// # Safety
///
/// `s` must point to a valid Service struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_close(s: *mut Service) {
    // SAFETY: Caller guarantees s points to valid Service.
    let srv = unsafe { &mut *s };
    srv.close();
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

    match srv.try_clone() {
        Ok(cloned) => {
            *out = cloned;
            0
        }
        Err(rc) => rc,
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

    match srv.try_clone_ex(tag) {
        Ok(cloned) => {
            *out = cloned;
            0
        }
        Err(rc) => rc,
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

    // For override services, we need to clone first (matching libnx behavior)
    if srv.own_handle == 0 && srv.object_id == 0 && srv.is_active() {
        match unsafe { service::clone_current_object_ex(srv.session, 0) } {
            Ok(new_handle) => {
                srv.session = new_handle;
                srv.own_handle = 1;
            }
            Err(rc) => return rc,
        }
    }

    match srv.convert_to_domain() {
        Ok(()) => 0,
        Err(rc) => rc,
    }
}

/// Returns whether a service is active (has valid session handle).
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sf__service_is_active(s: *const Service) -> bool {
    // SAFETY: We only read from the pointer and don't dereference if null.
    if s.is_null() {
        return false;
    }
    // SAFETY: Caller guarantees s points to valid Service.
    unsafe { (*s).is_active() }
}

/// Returns whether a service is an override service.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sf__service_is_override(s: *const Service) -> bool {
    if s.is_null() {
        return false;
    }
    // SAFETY: Caller guarantees s points to valid Service.
    unsafe { (*s).is_override() }
}

/// Returns whether a service is a domain.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sf__service_is_domain(s: *const Service) -> bool {
    if s.is_null() {
        return false;
    }
    // SAFETY: Caller guarantees s points to valid Service.
    unsafe { (*s).is_domain() }
}

/// Returns whether a service is a domain subservice.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sf__service_is_domain_subservice(s: *const Service) -> bool {
    if s.is_null() {
        return false;
    }
    // SAFETY: Caller guarantees s points to valid Service.
    unsafe { (*s).is_domain_subservice() }
}

/// Returns the object ID for a domain service.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sf__service_get_object_id(s: *const Service) -> u32 {
    if s.is_null() {
        return 0;
    }
    // SAFETY: Caller guarantees s points to valid Service.
    unsafe { (*s).object_id }
}
