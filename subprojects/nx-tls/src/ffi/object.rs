//! Reaching what a caller's service struct names.
//!
//! A context and a connection are each a libnx service struct, which [`nx_sf::ffi::Service`]
//! mirrors, and [`nx_sf::ffi::Service::as_domain_object`] addresses what one names without
//! adopting it. The C caller holds the close obligation for both, and nothing here may discharge
//! it: [`service_at`] is the exception, and exists so `sslContextClose` and `sslConnectionClose`
//! can perform the close the caller asked for.
//!
//! A struct naming no object is one that was never filled in, or one already closed. Upstream
//! tolerates the first case by dispatching on the plain session, because its own domain conversion
//! is allowed to fail. This does not: [`nx_service_ssl`] models both interfaces as domain objects,
//! so there is nothing to send a command through, and the caller is told what an uninitialized
//! service would report.

use core::ffi::c_void;

use nx_service_ssl::ffi::{
    ForeignSslConnection,
    ForeignSslContext,
};
use nx_sf::{
    ffi::Service,
    service::ForeignDomainObject,
};

/// Addresses the `ISslContext` the service struct at `ptr` names.
///
/// # Safety
///
/// `ptr` must be null or point to a readable libnx `SslContext`, whose first member is the service
/// struct this reads, so a pointer to one is a pointer to that.
pub(super) unsafe fn context_at(ptr: *mut c_void) -> Option<ForeignSslContext<'static>> {
    // SAFETY: the caller guarantees a readable service struct at `ptr`, or null.
    unsafe { object_at(ptr) }.map(ForeignSslContext::new)
}

/// Addresses the `ISslConnection` the service struct at `ptr` names.
///
/// # Safety
///
/// `ptr` must be null or point to a readable libnx `SslConnection`, whose first member is the
/// service struct this reads.
pub(super) unsafe fn connection_at(ptr: *mut c_void) -> Option<ForeignSslConnection<'static>> {
    // SAFETY: the caller guarantees a readable service struct at `ptr`, or null.
    unsafe { object_at(ptr) }.map(ForeignSslConnection::new)
}

/// Reads the libnx service struct at `ptr` and addresses the object it names.
///
/// Returns `None` for a null pointer and for a struct naming no object, which are the two ways a
/// caller can arrive without one.
///
/// # Safety
///
/// `ptr` must be null or point to a readable libnx service struct.
pub(super) unsafe fn object_at(ptr: *mut c_void) -> Option<ForeignDomainObject<'static>> {
    if ptr.is_null() {
        return None;
    }

    // SAFETY: the caller guarantees a readable service struct at a non-null `ptr`.
    let service = unsafe { *ptr.cast::<Service>() };
    service.as_domain_object()
}

/// Borrows the service struct at `ptr` for writing.
///
/// This is the one place that hands out a mutable view, because closing is the one operation that
/// changes the struct rather than reading it.
///
/// # Safety
///
/// `ptr` must be null or point to a readable and writable libnx service struct that no other
/// reference addresses for the lifetime of the returned one.
pub(super) unsafe fn service_at<'a>(ptr: *mut c_void) -> Option<&'a mut Service> {
    if ptr.is_null() {
        return None;
    }

    // SAFETY: the caller guarantees an exclusively-held, writable service struct at a non-null
    // `ptr` for the returned lifetime.
    Some(unsafe { &mut *ptr.cast::<Service>() })
}
