//! ISslService CMIF dispatch implementations.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainRef,
};

use crate::{
    proto,
    types::CreateContextIn,
};

/// Creates an SSL context. Returns the raw sub-object ID.
///
/// The close obligation is handed on rather than discharged: the caller
/// re-addresses the id through the long-lived parent domain.
pub(crate) fn create_context(
    domain: DomainRef<'_>,
    ssl_version: u32,
    system: bool,
) -> Result<u32, CreateContextError> {
    let input = CreateContextIn {
        ssl_version,
        _pad: 0,
        pid_placeholder: 0,
    };
    let cmd_id = if system {
        proto::CREATE_CONTEXT_SYSTEM
    } else {
        proto::CREATE_CONTEXT
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send(&mut ipc_buf)`
    // returns; viewing its `size_of::<CreateContextIn>()` bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<CreateContextIn>(),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let mut result = domain
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send_pid()
        .out_objects(1)
        .send(&mut ipc_buf)
        .map_err(CreateContextError::Dispatch)?;
    let object = result
        .take_object(0)
        .ok_or(CreateContextError::MissingObject)?;
    Ok(object.into_raw_object_id())
}

/// Error returned by [`create_context`].
#[derive(Debug, thiserror::Error)]
pub enum CreateContextError {
    /// IPC dispatch failed.
    #[error("failed to dispatch CreateContext")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected sub-object id.
    #[error("CreateContext response did not include the expected sub-object")]
    MissingObject,
}

/// Gets the context count.
pub(crate) fn get_context_count(domain: DomainRef<'_>) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::GET_CONTEXT_COUNT)
        .out_size(size_of::<u32>())
        .send(&mut ipc_buf)?;
    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Gets built-in certificates (pre-3.0.0, no output count).
pub(crate) fn get_certificates_legacy(
    domain: DomainRef<'_>,
    buffer: &mut [u8],
    ca_cert_ids: &[u32],
) -> Result<(), DispatchError> {
    // SAFETY: `ca_cert_ids` is a valid `&[u32]` slice; viewing it as bytes for
    // the IN buffer is sound.
    let ca_bytes = unsafe {
        core::slice::from_raw_parts(
            ca_cert_ids.as_ptr().cast::<u8>(),
            core::mem::size_of_val(ca_cert_ids),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::GET_CERTIFICATES)
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(ca_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Gets built-in certificates (3.0.0+, returns output count).
pub(crate) fn get_certificates(
    domain: DomainRef<'_>,
    buffer: &mut [u8],
    ca_cert_ids: &[u32],
) -> Result<u32, DispatchError> {
    // SAFETY: `ca_cert_ids` is a valid `&[u32]` slice; viewing it as bytes for
    // the IN buffer is sound.
    let ca_bytes = unsafe {
        core::slice::from_raw_parts(
            ca_cert_ids.as_ptr().cast::<u8>(),
            core::mem::size_of_val(ca_cert_ids),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::GET_CERTIFICATES)
        .out_size(size_of::<u32>())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(ca_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Gets the required buffer size for the given certificate IDs.
pub(crate) fn get_certificate_buf_size(
    domain: DomainRef<'_>,
    ca_cert_ids: &[u32],
) -> Result<u32, DispatchError> {
    // SAFETY: `ca_cert_ids` is a valid `&[u32]` slice; viewing it as bytes for
    // the IN buffer is sound.
    let ca_bytes = unsafe {
        core::slice::from_raw_parts(
            ca_cert_ids.as_ptr().cast::<u8>(),
            core::mem::size_of_val(ca_cert_ids),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::GET_CERTIFICATE_BUF_SIZE)
        .out_size(size_of::<u32>())
        .in_buffer(ca_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Sets the interface version (3.0.0+, internal).
pub(crate) fn set_interface_version(
    domain: DomainRef<'_>,
    version: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `version` is a `Copy` value on the stack, valid until `.send(&mut ipc_buf)`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const version).cast::<u8>(), size_of::<u32>()) };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::SET_INTERFACE_VERSION)
        .in_raw(in_bytes)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Flushes the session cache (5.0.0+).
pub(crate) fn flush_session_cache(
    domain: DomainRef<'_>,
    hostname: &[u8],
    option_type: u32,
) -> Result<u32, DispatchError> {
    // SAFETY: `option_type` is a `Copy` value on the stack, valid until `.send(&mut ipc_buf)`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const option_type).cast::<u8>(), size_of::<u32>())
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::FLUSH_SESSION_CACHE)
        .in_raw(in_bytes)
        .out_size(size_of::<u32>())
        .in_buffer(hostname, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Sets a debug option (6.0.0+).
pub(crate) fn set_debug_option(
    domain: DomainRef<'_>,
    debug_type: u32,
    buffer: &[u8],
) -> Result<(), DispatchError> {
    // SAFETY: `debug_type` is a `Copy` value on the stack, valid until `.send(&mut ipc_buf)`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const debug_type).cast::<u8>(), size_of::<u32>())
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::SET_DEBUG_OPTION)
        .in_raw(in_bytes)
        .in_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Gets a debug option (6.0.0+).
pub(crate) fn get_debug_option(
    domain: DomainRef<'_>,
    debug_type: u32,
    buffer: &mut [u8],
) -> Result<(), DispatchError> {
    // SAFETY: `debug_type` is a `Copy` value on the stack, valid until `.send(&mut ipc_buf)`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const debug_type).cast::<u8>(), size_of::<u32>())
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::GET_DEBUG_OPTION)
        .in_raw(in_bytes)
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Clears TLS 1.2 fallback flag (14.0.0+).
pub(crate) fn clear_tls12_fallback_flag(domain: DomainRef<'_>) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::CLEAR_TLS12_FALLBACK_FLAG)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Sets the thread core mask (15.0.0+, system only).
pub(crate) fn set_thread_core_mask(domain: DomainRef<'_>, mask: u64) -> Result<(), DispatchError> {
    // SAFETY: `mask` is a `Copy` value on the stack, valid until `.send(&mut ipc_buf)`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const mask).cast::<u8>(), size_of::<u64>()) };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::SET_THREAD_CORE_MASK)
        .in_raw(in_bytes)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Gets the thread core mask (15.0.0+, system only).
pub(crate) fn get_thread_core_mask(domain: DomainRef<'_>) -> Result<u64, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::GET_THREAD_CORE_MASK)
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)?;
    Ok(u64::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
        result.data[4],
        result.data[5],
        result.data[6],
        result.data[7],
    ]))
}
