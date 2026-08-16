//! Bringing the `ssl` service up and down, and the commands the service itself answers.
//!
//! ## Which service variant a program gets
//!
//! The default one, always: see [`selects_system`].

use core::ffi::{
    c_char,
    c_void,
};

use nx_service_ssl::{
    DebugOptionType,
    FlushSessionCacheOptionType,
    SessionCount,
    SslVersion,
};
use nx_sf::{
    error::{
        LibnxError,
        ResultCode,
        ToResultCode as _,
        libnx_error,
    },
    ffi::Service,
};

use super::{
    buffer,
    firmware,
    object,
    result,
    session,
};

/// Brings the `ssl` service up, or records another caller for one already up.
///
/// `num_sessions` sizes the pool of IPC sessions commands run on, and must be `1..=4`.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_tls__sslInitialize(num_sessions: u32) -> ResultCode {
    let Ok(sessions) = SessionCount::try_from(num_sessions) else {
        return result::bad_input();
    };

    match session::initialize(sessions, selects_system()) {
        Ok(()) => result::OK,
        Err(err) => err.to_rc(),
    }
}

/// Releases one caller's claim on the service, bringing it down when it was the last.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_tls__sslExit() {
    session::exit();
}

/// Hands out the service session, for a program sending commands this crate does not carry.
///
/// The pointer is never null: before initialization and after the last exit it names a zeroed
/// struct, which is what upstream leaves behind and what `serviceIsActive` reads as inactive.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_tls__sslGetServiceSession() -> *mut Service {
    session::root_session()
}

/// Creates an SSL context, writing it into the caller's struct.
///
/// # Safety
///
/// `context` must point to a writable libnx `SslContext`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslCreateContext(
    context: *mut c_void,
    ssl_version: u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a writable `SslContext`, whose first member is the service
    // struct this writes, and which nothing else addresses for the length of this call.
    let Some(out) = (unsafe { object::service_at(context) }) else {
        return result::bad_input();
    };

    // The context is described while the service is still borrowed, because it borrows the
    // service in turn. What comes out is the plain struct C holds, which borrows nothing.
    //
    // That struct carries the sole closer for the new context, so writing it into the caller's
    // struct is what hands the close on: from here the C caller owns it, and `sslContextClose` is
    // what discharges it.
    let created = session::with_service(|service| {
        service
            .create_context(SslVersion::from_bits_retain(ssl_version))
            .map(|context| Service::from(context.into_object()))
    });

    match created {
        None => result::not_initialized(),
        Some(Err(err)) => err.to_rc(),
        Some(Ok(context)) => {
            *out = context;
            result::OK
        }
    }
}

/// Reports how many contexts exist.
///
/// # Safety
///
/// `out` must be null or point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslGetContextCount(out: *mut u32) -> ResultCode {
    let Some(count) = session::with_service(nx_service_ssl::SslService::get_context_count) else {
        return result::not_initialized();
    };

    match count {
        Ok(count) => {
            // SAFETY: the caller guarantees a writable `u32` at `out`, or null.
            unsafe { buffer::write_out(out, count) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Reads the built-in certificates the caller named.
///
/// # Safety
///
/// `buffer` must point to `size` writable bytes, `ca_cert_ids` to `count` readable `u32`s, and
/// `total_out` must be null or point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslGetCertificates(
    buffer: *mut c_void,
    size: u32,
    ca_cert_ids: *const u32,
    count: u32,
    total_out: *mut u32,
) -> ResultCode {
    if buffer.is_null() || size == 0 || ca_cert_ids.is_null() || count == 0 {
        return result::bad_input();
    }

    // SAFETY: the caller guarantees `count` readable `u32`s at a non-null `ca_cert_ids`.
    let ids = unsafe { core::slice::from_raw_parts(ca_cert_ids, count as usize) };
    // SAFETY: the caller guarantees `size` writable bytes at a non-null `buffer`, exclusively held
    // for this call.
    let bytes = unsafe { buffer::bytes_mut(buffer, size) };

    // Before `[3.0.0]` the command reports no count, and every id the caller asked for was
    // written, so the number it asked for is the answer.
    let written = session::with_service(|service| {
        if firmware::offers_certificate_count() {
            service.get_certificates(bytes, ids)
        } else {
            service.get_certificates_legacy(bytes, ids).map(|()| count)
        }
    });

    let written = match written {
        None => return result::not_initialized(),
        Some(Err(err)) => return err.to_rc(),
        Some(Ok(written)) => written,
    };

    // SAFETY: the buffer is still the caller's `size` writable bytes, and the entries the service
    // wrote into it are what this rewrites.
    if let Err(err) = unsafe { anchor_certificates(buffer, size, written) } {
        return err;
    }

    // SAFETY: the caller guarantees a writable `u32` at `total_out`, or null.
    unsafe { buffer::write_out(total_out, written) };
    result::OK
}

/// Reports how large a buffer the named certificates need.
///
/// # Safety
///
/// `ca_cert_ids` must point to `count` readable `u32`s, and `out` must be null or point to a
/// writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslGetCertificateBufSize(
    ca_cert_ids: *const u32,
    count: u32,
    out: *mut u32,
) -> ResultCode {
    if ca_cert_ids.is_null() || count == 0 {
        return result::bad_input();
    }

    // SAFETY: the caller guarantees `count` readable `u32`s at a non-null `ca_cert_ids`.
    let ids = unsafe { core::slice::from_raw_parts(ca_cert_ids, count as usize) };

    let Some(size) = session::with_service(|service| service.get_certificate_buf_size(ids)) else {
        return result::not_initialized();
    };

    match size {
        Ok(size) => {
            // SAFETY: the caller guarantees a writable `u32` at `out`, or null.
            unsafe { buffer::write_out(out, size) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Flushes the service-wide session cache (5.0.0+).
///
/// # Safety
///
/// `host` must be null or point to at most `host_len` readable bytes, and `out` must be null or
/// point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslFlushSessionCache(
    host: *const c_char,
    host_len: usize,
    option_type: u32,
    out: *mut u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a writable `u32` at `out`, or null. The count is cleared
    // first, so a caller that ignores the result code does not read a stale one.
    unsafe { buffer::write_out(out, 0) };

    // SAFETY: the caller guarantees at most `host_len` readable bytes at `host`, or null.
    let host = match unsafe { flushed_host(host, host_len, option_type) } {
        Ok(host) => host,
        Err(err) => return err,
    };

    if !firmware::offers_session_cache_flush() {
        return result::incompat_sys_ver();
    }

    let option_type = if option_type == FlushSessionCacheOptionType::AllHosts as u32 {
        FlushSessionCacheOptionType::AllHosts
    } else {
        FlushSessionCacheOptionType::SingleHost
    };

    let Some(flushed) =
        session::with_service(|service| service.flush_session_cache(host, option_type))
    else {
        return result::not_initialized();
    };

    match flushed {
        Ok(count) => {
            // SAFETY: the caller guarantees a writable `u32` at `out`, or null.
            unsafe { buffer::write_out(out, count) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Sets a debug option (6.0.0+).
///
/// # Safety
///
/// `buffer` must point to `size` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslSetDebugOption(
    buffer: *const c_void,
    size: usize,
    option_type: u32,
) -> ResultCode {
    if !firmware::offers_debug_option() {
        return result::incompat_sys_ver();
    }

    let Ok(option_type) = DebugOptionType::try_from(option_type) else {
        return result::bad_input();
    };

    if buffer.is_null() {
        return result::bad_input();
    }

    // SAFETY: the caller guarantees `size` readable bytes at a non-null `buffer`.
    let bytes = unsafe { buffer::bytes(buffer, size as u32) };

    let sent = session::with_service(|service| service.set_debug_option(option_type, bytes));

    sent.map_or_else(result::not_initialized, result::report)
}

/// Reads a debug option (6.0.0+).
///
/// # Safety
///
/// `buffer` must point to `size` writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslGetDebugOption(
    buffer: *mut c_void,
    size: usize,
    option_type: u32,
) -> ResultCode {
    if !firmware::offers_debug_option() {
        return result::incompat_sys_ver();
    }

    let Ok(option_type) = DebugOptionType::try_from(option_type) else {
        return result::bad_input();
    };

    // SAFETY: the caller guarantees `size` writable bytes at `buffer`, exclusively held for this
    // call.
    let bytes = unsafe { buffer::bytes_mut(buffer, size as u32) };

    let read = session::with_service(|service| service.get_debug_option(option_type, bytes));

    read.map_or_else(result::not_initialized, result::report)
}

/// Clears the TLS 1.2 fallback flag (14.0.0+).
#[unsafe(no_mangle)]
pub extern "C" fn __nx_tls__sslClearTls12FallbackFlag() -> ResultCode {
    if !firmware::offers_tls12_fallback_flag() {
        return result::incompat_sys_ver();
    }

    session::with_service(nx_service_ssl::SslService::clear_tls12_fallback_flag)
        .map_or_else(result::not_initialized, result::report)
}

/// Sets the service's thread core mask (15.0.0+, system service only).
#[unsafe(no_mangle)]
pub extern "C" fn __nx_tls__sslSetThreadCoreMask(mask: u64) -> ResultCode {
    if !firmware::offers_system_interface() {
        return result::incompat_sys_ver();
    }

    if !selects_system() {
        return result::not_initialized();
    }

    session::with_service(|service| service.set_thread_core_mask(mask))
        .map_or_else(result::not_initialized, result::report)
}

/// Reads the service's thread core mask (15.0.0+, system service only).
///
/// # Safety
///
/// `out` must be null or point to a writable `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslGetThreadCoreMask(out: *mut u64) -> ResultCode {
    if !firmware::offers_system_interface() {
        return result::incompat_sys_ver();
    }

    if !selects_system() {
        return result::not_initialized();
    }

    let Some(mask) = session::with_service(nx_service_ssl::SslService::get_thread_core_mask) else {
        return result::not_initialized();
    };

    match mask {
        Ok(mask) => {
            // SAFETY: the caller guarantees a writable `u64` at `out`, or null.
            unsafe { buffer::write_out(out, mask) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// A built-in certificate, as the service describes one.
///
/// `cert_data` arrives as an **offset** from the start of the buffer, and the C API hands out a
/// pointer, so [`anchor_certificates`] rewrites the field in place. That is the whole reason this
/// struct is named here rather than in [`nx_service_ssl`]: the offset is what the service sends,
/// and the pointer is what C expects to read.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct BuiltInCertificateInfo {
    cert_id: u32,
    status: u32,
    cert_size: u64,
    cert_data: *mut u8,
}

/// Rewrites each certificate's offset into a pointer the caller can dereference.
///
/// Every offset is checked against the buffer it indexes before it becomes a pointer, so a service
/// answer that does not fit is reported rather than handed to C as an address just outside.
///
/// # Safety
///
/// `buffer` must point to `size` writable bytes holding `written` [`BuiltInCertificateInfo`]
/// entries, which is what the command writes there.
unsafe fn anchor_certificates(
    buffer: *mut c_void,
    size: u32,
    written: u32,
) -> Result<(), ResultCode> {
    let base = buffer.cast::<u8>();

    for index in 0..written as usize {
        // SAFETY: the caller guarantees `written` entries at `buffer`.
        let entry = unsafe { &mut *base.cast::<BuiltInCertificateInfo>().add(index) };

        let offset = entry.cert_data as u64;
        let end = offset.checked_add(entry.cert_size);
        if offset >= u64::from(size) || entry.cert_size >= u64::from(size) {
            return Err(libnx_error(LibnxError::ShouldNotHappen));
        }
        if end.is_none_or(|end| end > u64::from(size)) {
            return Err(libnx_error(LibnxError::ShouldNotHappen));
        }

        // SAFETY: `offset` was just checked to land inside the caller's `size` bytes, so the
        // result is a pointer into that same allocation.
        entry.cert_data = unsafe { base.add(offset as usize) };
    }

    Ok(())
}

/// Reads the host name a cache flush names, as the option type requires.
///
/// The two option types want opposite things, and the C API passes both through one pointer. This
/// is where that stops: flushing one host needs a non-empty name, flushing all of them needs no
/// name at all, and anything else is a caller that has confused the two.
///
/// The name is sent with its terminator, because that is what the service reads.
///
/// # Safety
///
/// `host` must be null or point to at most `host_len` readable bytes.
unsafe fn flushed_host<'a>(
    host: *const c_char,
    host_len: usize,
    option_type: u32,
) -> Result<&'a [u8], ResultCode> {
    if option_type == FlushSessionCacheOptionType::AllHosts as u32 {
        return if host.is_null() && host_len == 0 {
            Ok(&[])
        } else {
            Err(result::bad_input())
        };
    }

    if option_type != FlushSessionCacheOptionType::SingleHost as u32 || host.is_null() {
        return Err(result::bad_input());
    }

    // SAFETY: the caller guarantees `host_len` readable bytes at a non-null `host`.
    let bounded = unsafe { core::slice::from_raw_parts(host.cast::<u8>(), host_len) };
    let name_len = bounded
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(host_len);
    if name_len == 0 {
        return Err(result::bad_input());
    }

    Ok(&bounded[..name_len + 1])
}

/// Whether this program gets the system service.
///
/// Always false. Upstream reads a weak `__nx_ssl_service_type` global a program may define to ask
/// for `ssl:s`, and declaring a weak *undefined* symbol from Rust needs an unstable compiler
/// feature this workspace does not enable. So the choice is not offered rather than offered and
/// silently ignored, and the two commands that exist only on the system variant answer through
/// this rather than assuming.
///
/// `ssl:s` needs permissions homebrew does not have, so no program this workspace targets can use
/// it either way.
// TODO: offer the choice again if a way to read the knob without an unstable feature appears, or
//  if a Rust-side equivalent is introduced.
fn selects_system() -> bool {
    false
}
