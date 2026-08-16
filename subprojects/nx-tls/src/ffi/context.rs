//! The commands an `ISslContext` answers, from C.
//!
//! Each takes a pointer to a libnx `SslContext`, which is a service struct and nothing else, and
//! addresses the object it names. The C caller created the context through
//! [`sslCreateContext`](super::service::__nx_tls__sslCreateContext) and closes it through
//! [`sslContextClose`], so nothing here closes one except that.

use core::ffi::{
    c_char,
    c_void,
};

use nx_service_ssl::{
    CertificateFormat,
    ConnectionKind,
    ContextOption,
    InternalPki,
    KeyAndCertParams,
};
use nx_sf::error::{
    ResultCode,
    ToResultCode as _,
};

use super::{
    buffer,
    firmware,
    object,
    result,
    session,
};

/// Closes a context, and the connections the server opened under it.
///
/// # Safety
///
/// `context` must be null or point to a writable libnx `SslContext`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslContextClose(context: *mut c_void) {
    // SAFETY: the caller guarantees a writable `SslContext`, or null, which nothing else addresses
    // for the length of this call.
    if let Some(service) = unsafe { object::service_at(context) } {
        service.close();
    }
}

/// Sets a context option.
///
/// # Safety
///
/// `context` must be null or point to a readable libnx `SslContext`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslContextSetOption(
    context: *mut c_void,
    option: u32,
    value: i32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslContext`, or null.
    let Some(context) = (unsafe { object::context_at(context) }) else {
        return result::not_initialized();
    };

    let Ok(option) = ContextOption::try_from(option) else {
        return result::bad_input();
    };

    result::report(context.set_option(option, value))
}

/// Reads a context option.
///
/// # Safety
///
/// `context` must be null or point to a readable libnx `SslContext`, and `out` must be null or
/// point to a writable `i32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslContextGetOption(
    context: *mut c_void,
    option: u32,
    out: *mut i32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslContext`, or null.
    let Some(context) = (unsafe { object::context_at(context) }) else {
        return result::not_initialized();
    };

    let Ok(option) = ContextOption::try_from(option) else {
        return result::bad_input();
    };

    match context.get_option(option) {
        Ok(value) => {
            // SAFETY: the caller guarantees a writable `i32` at `out`, or null.
            unsafe { buffer::write_out(out, value) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Creates a connection under a context, writing it into the caller's struct.
///
/// # Safety
///
/// `context` must be null or point to a readable libnx `SslContext`, and `connection` must point
/// to a writable libnx `SslConnection`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslContextCreateConnection(
    context: *mut c_void,
    connection: *mut c_void,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslContext` and a writable `SslConnection`, or
    // null, neither addressed elsewhere for the length of this call.
    unsafe { create_connection(context, connection, ConnectionKind::Application) }
}

/// Creates a connection for system under a context (15.0.0+, system service only).
///
/// # Safety
///
/// As [`__nx_tls__sslContextCreateConnection`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslContextCreateConnectionForSystem(
    context: *mut c_void,
    connection: *mut c_void,
) -> ResultCode {
    if !firmware::offers_system_interface() {
        return result::incompat_sys_ver();
    }

    // SAFETY: as the default counterpart above.
    unsafe { create_connection(context, connection, ConnectionKind::System) }
}

/// Reports how many connections exist under a context.
///
/// # Safety
///
/// `context` must be null or point to a readable libnx `SslContext`, and `out` must be null or
/// point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslContextGetConnectionCount(
    context: *mut c_void,
    out: *mut u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslContext`, or null.
    let Some(context) = (unsafe { object::context_at(context) }) else {
        return result::not_initialized();
    };

    match context.get_connection_count() {
        Ok(count) => {
            // SAFETY: the caller guarantees a writable `u32` at `out`, or null.
            unsafe { buffer::write_out(out, count) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Imports server PKI certificates, reporting the id the service assigned them.
///
/// # Safety
///
/// `context` must be null or point to a readable libnx `SslContext`, `buffer` to `size` readable
/// bytes, and `id` must be null or point to a writable `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslContextImportServerPki(
    context: *mut c_void,
    buffer: *const c_void,
    size: u32,
    format: u32,
    id: *mut u64,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslContext`, or null.
    let Some(context) = (unsafe { object::context_at(context) }) else {
        return result::not_initialized();
    };

    let Ok(format) = CertificateFormat::try_from(format) else {
        return result::bad_input();
    };

    if buffer.is_null() {
        return result::bad_input();
    }

    // SAFETY: the caller guarantees `size` readable bytes at a non-null `buffer`.
    let cert_data = unsafe { buffer::bytes(buffer, size) };

    match context.import_server_pki(cert_data, format) {
        Ok(assigned) => {
            // SAFETY: the caller guarantees a writable `u64` at `id`, or null.
            unsafe { buffer::write_out(id, assigned) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Imports a client PKI, reporting the id the service assigned it.
///
/// # Safety
///
/// `context` must be null or point to a readable libnx `SslContext`, `pkcs12` to `pkcs12_size`
/// readable bytes, `password` to `password_size` readable bytes, and `id` must be null or point to
/// a writable `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslContextImportClientPki(
    context: *mut c_void,
    pkcs12: *const c_void,
    pkcs12_size: u32,
    password: *const c_char,
    password_size: u32,
    id: *mut u64,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslContext`, or null.
    let Some(context) = (unsafe { object::context_at(context) }) else {
        return result::not_initialized();
    };

    // A password and its length have to agree: either both are there or neither is. One without
    // the other is a caller that has lost track of which, and upstream rejects it too.
    if pkcs12.is_null() || password.is_null() != (password_size == 0) {
        return result::bad_input();
    }

    // SAFETY: the caller guarantees the two buffers at the two lengths, either of which may be
    // absent, which `bytes` answers with an empty slice.
    let (pkcs12, password) = unsafe {
        (
            buffer::bytes(pkcs12, pkcs12_size),
            buffer::bytes(password.cast(), password_size),
        )
    };

    match context.import_client_pki(pkcs12, password) {
        Ok(assigned) => {
            // SAFETY: the caller guarantees a writable `u64` at `id`, or null.
            unsafe { buffer::write_out(id, assigned) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Removes whatever the id names: a server PKI, a client PKI, or a CRL.
///
/// # Safety
///
/// `context` must be null or point to a readable libnx `SslContext`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslContextRemovePki(
    context: *mut c_void,
    id: u64,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslContext`, or null.
    let Some(context) = (unsafe { object::context_at(context) }) else {
        return result::not_initialized();
    };

    // A CRL is only somewhere the id could be from `[3.0.0]`, so on older firmware the search
    // stops after the two PKI commands rather than sending one the service does not answer.
    match context.remove_pki(id, firmware::offers_certificate_count()) {
        Ok(()) => result::OK,
        Err(err) => err.0.to_rc(),
    }
}

/// Registers an internal PKI, reporting the id the service assigned it.
///
/// # Safety
///
/// `context` must be null or point to a readable libnx `SslContext`, and `id` must be null or
/// point to a writable `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslContextRegisterInternalPki(
    context: *mut c_void,
    internal_pki: u32,
    id: *mut u64,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslContext`, or null.
    let Some(context) = (unsafe { object::context_at(context) }) else {
        return result::not_initialized();
    };

    let Ok(internal_pki) = InternalPki::try_from(internal_pki) else {
        return result::bad_input();
    };

    match context.register_internal_pki(internal_pki) {
        Ok(assigned) => {
            // SAFETY: the caller guarantees a writable `u64` at `id`, or null.
            unsafe { buffer::write_out(id, assigned) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Adds a policy OID string.
///
/// # Safety
///
/// `context` must be null or point to a readable libnx `SslContext`, and `oid` to `oid_len`
/// readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslContextAddPolicyOid(
    context: *mut c_void,
    oid: *const c_char,
    oid_len: u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslContext`, or null.
    let Some(context) = (unsafe { object::context_at(context) }) else {
        return result::not_initialized();
    };

    // The service reads the string out of a fixed field, so a longer one is rejected here rather
    // than sent and truncated.
    const MAX_OID_LEN: u32 = 0xFF;
    if oid.is_null() || oid_len > MAX_OID_LEN {
        return result::bad_input();
    }

    // SAFETY: the caller guarantees `oid_len` readable bytes at a non-null `oid`.
    let oid = unsafe { buffer::bytes(oid.cast(), oid_len) };

    result::report(context.add_policy_oid(oid))
}

/// Imports a CRL, reporting the id the service assigned it (3.0.0+).
///
/// # Safety
///
/// `context` must be null or point to a readable libnx `SslContext`, `buffer` to `size` readable
/// bytes, and `id` must be null or point to a writable `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslContextImportCrl(
    context: *mut c_void,
    buffer: *const c_void,
    size: u32,
    id: *mut u64,
) -> ResultCode {
    if !firmware::offers_certificate_count() {
        return result::incompat_sys_ver();
    }

    // SAFETY: the caller guarantees a readable `SslContext`, or null.
    let Some(context) = (unsafe { object::context_at(context) }) else {
        return result::not_initialized();
    };

    if buffer.is_null() {
        return result::bad_input();
    }

    // SAFETY: the caller guarantees `size` readable bytes at a non-null `buffer`.
    let crl_data = unsafe { buffer::bytes(buffer, size) };

    match context.import_crl(crl_data) {
        Ok(assigned) => {
            // SAFETY: the caller guarantees a writable `u64` at `id`, or null.
            unsafe { buffer::write_out(id, assigned) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Imports a client certificate and its key, reporting the id assigned (16.0.0+).
///
/// # Safety
///
/// `context` must be null or point to a readable libnx `SslContext`, `cert` to `cert_size`
/// readable bytes, `key` to `key_size` readable bytes, and `id` must be null or point to a
/// writable `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslContextImportClientCertKeyPki(
    context: *mut c_void,
    cert: *const c_void,
    cert_size: u32,
    key: *const c_void,
    key_size: u32,
    format: u32,
    id: *mut u64,
) -> ResultCode {
    if !firmware::offers_dtls() {
        return result::incompat_sys_ver();
    }

    // SAFETY: the caller guarantees a readable `SslContext`, or null.
    let Some(context) = (unsafe { object::context_at(context) }) else {
        return result::not_initialized();
    };

    let Ok(format) = CertificateFormat::try_from(format) else {
        return result::bad_input();
    };

    if cert.is_null() || key.is_null() {
        return result::bad_input();
    }

    // SAFETY: the caller guarantees the two buffers at the two lengths.
    let (cert, key) = unsafe { (buffer::bytes(cert, cert_size), buffer::bytes(key, key_size)) };

    match context.import_client_cert_key_pki(cert, key, format) {
        Ok(assigned) => {
            // SAFETY: the caller guarantees a writable `u64` at `id`, or null.
            unsafe { buffer::write_out(id, assigned) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Generates a private key and a certificate for it (16.0.0+).
///
/// # Safety
///
/// `context` must be null or point to a readable libnx `SslContext`, `cert` to `cert_size`
/// writable bytes, `key` to `key_size` writable bytes, `params` to a readable
/// [`KeyAndCertParams`], and each out-pointer must be null or point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslContextGeneratePrivateKeyAndCert(
    context: *mut c_void,
    cert: *mut c_void,
    cert_size: u32,
    key: *mut c_void,
    key_size: u32,
    val: u32,
    params: *const KeyAndCertParams,
    out_cert_size: *mut u32,
    out_key_size: *mut u32,
) -> ResultCode {
    if !firmware::offers_dtls() {
        return result::incompat_sys_ver();
    }

    // SAFETY: the caller guarantees a readable `SslContext`, or null.
    let Some(context) = (unsafe { object::context_at(context) }) else {
        return result::not_initialized();
    };

    // The service accepts one value here, and rejecting anything else at the boundary is cheaper
    // than sending a request that cannot succeed.
    const REQUIRED_VAL: u32 = 1;
    if cert.is_null() || key.is_null() || params.is_null() || val != REQUIRED_VAL {
        return result::bad_input();
    }

    // SAFETY: the caller guarantees a readable `KeyAndCertParams` at a non-null `params`.
    let params = unsafe { &*params };
    // SAFETY: the caller guarantees the two buffers at the two lengths, exclusively held for this
    // call.
    let (cert, key) = unsafe {
        (
            buffer::bytes_mut(cert, cert_size),
            buffer::bytes_mut(key, key_size),
        )
    };

    match context.generate_private_key_and_cert(cert, key, params) {
        Ok((written_cert, written_key)) => {
            // SAFETY: the caller guarantees writable `u32`s at the two out-pointers, or null.
            unsafe {
                buffer::write_out(out_cert_size, written_cert);
                buffer::write_out(out_key_size, written_key);
            }
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Runs one of the two connection-creating commands and hands the result to C.
///
/// The pair differ only in which command they send, so the way the context is read, the object is
/// created and the caller's struct is written are written once here.
///
/// The object comes from the service rather than from the context view, because adopting what the
/// reply carries is the domain owner's job: the C caller holds the context, but the domain under
/// it is this process's.
///
/// # Safety
///
/// `context` must be null or point to a readable libnx `SslContext`, and `connection` must point
/// to a writable libnx `SslConnection` that nothing else addresses for the length of this call.
unsafe fn create_connection(
    context: *mut c_void,
    connection: *mut c_void,
    kind: ConnectionKind,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslContext`, or null.
    let Some(object) = (unsafe { object::object_at(context) }) else {
        return result::not_initialized();
    };

    // SAFETY: the caller guarantees an exclusively-held, writable `SslConnection`.
    let Some(out) = (unsafe { object::service_at(connection) }) else {
        return result::bad_input();
    };

    // Described while the service is still borrowed, for the reason `sslCreateContext` gives.
    // Writing the struct into the caller's is what hands the close on: from here the C caller owns
    // the connection, and `sslConnectionClose` is what discharges it.
    let created = session::with_service(|service| {
        service
            .create_connection_under(object.object_id(), kind)
            .map(nx_sf::ffi::Service::from)
    });

    match created {
        None => result::not_initialized(),
        Some(Err(err)) => err.to_rc(),
        Some(Ok(connection)) => {
            *out = connection;
            result::OK
        }
    }
}
