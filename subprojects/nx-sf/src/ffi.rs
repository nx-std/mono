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

use core::mem;

use nx_svc::{error::ToRawResultCode, ipc::Handle as SessionHandle, raw::INVALID_HANDLE};

use crate::{
    cmif,
    cmif::ObjectId,
    service::{
        self, CloneObjectError, CloneObjectExError, Service, ServiceConvertToDomainError,
        TryCloneError, TryCloneExError,
    },
};

/// Generic error code for FFI when no specific result code is available.
const GENERIC_ERROR: u32 = 0xFFFF;

/// Creates a service object from an IPC session handle.
///
/// # Safety
///
/// `s` must point to valid, writable memory for a Service struct.
/// `h` must be a valid IPC session handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_create(s: *mut Service, h: u32) {
    // SAFETY: h is a valid handle per caller contract.
    let handle = unsafe { SessionHandle::from_raw(h) };

    // SAFETY: Caller guarantees s points to valid memory.
    unsafe { *s = Service::new(handle) };
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
        let handle = unsafe { SessionHandle::from_raw(h) };
        // SAFETY: s points to valid memory.
        unsafe { *s = Service::new_subservice(parent, handle) };
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

    if let Some(object_id) = ObjectId::new(object_id) {
        // SAFETY: s points to valid memory.
        unsafe { *s = Service::new_domain_subservice(parent, object_id) };
    } else {
        // SAFETY: Service is repr(C) and can be zero-initialized for FFI.
        unsafe { *s = mem::zeroed() };
    }
}

/// Closes a service and releases its resources.
///
/// # Safety
///
/// `s` must point to a valid Service struct.
/// After this call, the Service at `s` is zeroed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sf__service_close(s: *mut Service) {
    // SAFETY: Caller guarantees s points to valid Service.
    let srv = unsafe { *s };

    // Close consumes the service
    srv.close();

    // Zero the memory (service is now invalid)
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

    match srv.try_clone() {
        Ok(cloned) => {
            *out = cloned;
            0
        }
        Err(TryCloneError(err)) => clone_error_to_rc(err),
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
        Err(TryCloneExError(err)) => clone_object_ex_error_to_rc(err),
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
    // Override services have own_handle == 0 and object_id == 0
    if srv.is_override() {
        match service::clone_current_object_ex(srv.session, 0) {
            Ok(new_handle) => {
                srv.session = new_handle;
                srv.own_handle = 1;
            }
            Err(err) => return clone_object_ex_error_to_rc(err),
        }
    }

    match srv.convert_to_domain() {
        Ok(()) => 0,
        Err(ServiceConvertToDomainError(err)) => convert_to_domain_error_to_rc(err),
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

/// Converts a clone object error to a raw result code for FFI.
fn clone_error_to_rc(err: CloneObjectError) -> u32 {
    match err {
        CloneObjectError::SendRequest(e) => e.to_rc(),
        CloneObjectError::ParseResponse(e) => parse_response_error_to_rc(e),
        CloneObjectError::MissingHandle => GENERIC_ERROR,
    }
}

/// Converts a parse response error to a raw result code.
fn parse_response_error_to_rc(err: cmif::ParseResponseError) -> u32 {
    match err {
        cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
        cmif::ParseResponseError::ServiceError(code) => code,
    }
}

/// Converts a clone object ex error to a raw result code for FFI.
fn clone_object_ex_error_to_rc(err: CloneObjectExError) -> u32 {
    match err {
        CloneObjectExError::SendRequest(e) => e.to_rc(),
        CloneObjectExError::ParseResponse(e) => parse_response_error_to_rc(e),
        CloneObjectExError::MissingHandle => GENERIC_ERROR,
    }
}

/// Converts a convert to domain error to a raw result code for FFI.
fn convert_to_domain_error_to_rc(err: service::ConvertToDomainError) -> u32 {
    match err {
        service::ConvertToDomainError::SendRequest(e) => e.to_rc(),
        service::ConvertToDomainError::ParseResponse(e) => parse_response_error_to_rc(e),
    }
}
