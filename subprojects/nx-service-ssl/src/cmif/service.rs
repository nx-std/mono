//! ISslService CMIF dispatch implementations.

use core::mem::{ManuallyDrop, size_of};

use nx_sf::service::{BufferAttr, DispatchError, Domain};

use crate::{proto, types::CreateContextIn};

/// Creates an SSL context. Returns the raw sub-object ID.
///
/// The freshly minted `DomainObject` is wrapped in `ManuallyDrop` so the
/// server-side object outlives this call; the service wrapper re-opens it
/// per request.
pub(crate) fn create_context(
    domain: &Domain,
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
    // SAFETY: `input` lives on the stack until `.send()` returns.
    let mut result = unsafe {
        domain
            .dispatch(cmd_id)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<CreateContextIn>(),
            )
            .send_pid()
            .out_objects(1)
            .send()
            .map_err(CreateContextError::Dispatch)?
    };
    let object = result
        .take_object(0)
        .ok_or(CreateContextError::MissingObject)?;
    Ok(ManuallyDrop::new(object).object_id().to_raw())
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
pub(crate) fn get_context_count(domain: &Domain) -> Result<u32, DispatchError> {
    let result = domain
        .dispatch(proto::GET_CONTEXT_COUNT)
        .out_size(size_of::<u32>())
        .send()?;
    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Gets built-in certificates (pre-3.0.0, no output count).
pub(crate) fn get_certificates_legacy(
    domain: &Domain,
    buffer: &mut [u8],
    ca_cert_ids: &[u32],
) -> Result<(), DispatchError> {
    domain
        .dispatch(proto::GET_CERTIFICATES)
        .buffer(
            buffer.as_mut_ptr(),
            buffer.len(),
            BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .buffer(
            ca_cert_ids.as_ptr().cast::<u8>(),
            core::mem::size_of_val(ca_cert_ids),
            BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send()
        .map(|_| ())
}

/// Gets built-in certificates (3.0.0+, returns output count).
pub(crate) fn get_certificates(
    domain: &Domain,
    buffer: &mut [u8],
    ca_cert_ids: &[u32],
) -> Result<u32, DispatchError> {
    let result = domain
        .dispatch(proto::GET_CERTIFICATES)
        .out_size(size_of::<u32>())
        .buffer(
            buffer.as_mut_ptr(),
            buffer.len(),
            BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .buffer(
            ca_cert_ids.as_ptr().cast::<u8>(),
            core::mem::size_of_val(ca_cert_ids),
            BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send()?;
    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Gets the required buffer size for the given certificate IDs.
pub(crate) fn get_certificate_buf_size(
    domain: &Domain,
    ca_cert_ids: &[u32],
) -> Result<u32, DispatchError> {
    let result = domain
        .dispatch(proto::GET_CERTIFICATE_BUF_SIZE)
        .out_size(size_of::<u32>())
        .buffer(
            ca_cert_ids.as_ptr().cast::<u8>(),
            core::mem::size_of_val(ca_cert_ids),
            BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send()?;
    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Sets the interface version (3.0.0+, internal).
pub(crate) fn set_interface_version(domain: &Domain, version: u32) -> Result<(), DispatchError> {
    // SAFETY: `version` lives on the stack until `.send()` returns.
    unsafe {
        domain
            .dispatch(proto::SET_INTERFACE_VERSION)
            .in_raw((&raw const version).cast::<u8>(), size_of::<u32>())
            .send()
            .map(|_| ())
    }
}

/// Flushes the session cache (5.0.0+).
pub(crate) fn flush_session_cache(
    domain: &Domain,
    hostname: &[u8],
    option_type: u32,
) -> Result<u32, DispatchError> {
    let result = unsafe {
        domain
            .dispatch(proto::FLUSH_SESSION_CACHE)
            .in_raw((&raw const option_type).cast::<u8>(), size_of::<u32>())
            .out_size(size_of::<u32>())
            .buffer(
                hostname.as_ptr(),
                hostname.len(),
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()?
    };
    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Sets a debug option (6.0.0+).
pub(crate) fn set_debug_option(
    domain: &Domain,
    debug_type: u32,
    buffer: &[u8],
) -> Result<(), DispatchError> {
    // SAFETY: `debug_type` lives on the stack until `.send()` returns.
    unsafe {
        domain
            .dispatch(proto::SET_DEBUG_OPTION)
            .in_raw((&raw const debug_type).cast::<u8>(), size_of::<u32>())
            .buffer(
                buffer.as_ptr(),
                buffer.len(),
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map(|_| ())
    }
}

/// Gets a debug option (6.0.0+).
pub(crate) fn get_debug_option(
    domain: &Domain,
    debug_type: u32,
    buffer: &mut [u8],
) -> Result<(), DispatchError> {
    // SAFETY: `debug_type` lives on the stack until `.send()` returns.
    unsafe {
        domain
            .dispatch(proto::GET_DEBUG_OPTION)
            .in_raw((&raw const debug_type).cast::<u8>(), size_of::<u32>())
            .buffer(
                buffer.as_mut_ptr(),
                buffer.len(),
                BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map(|_| ())
    }
}

/// Clears TLS 1.2 fallback flag (14.0.0+).
pub(crate) fn clear_tls12_fallback_flag(domain: &Domain) -> Result<(), DispatchError> {
    domain
        .dispatch(proto::CLEAR_TLS12_FALLBACK_FLAG)
        .send()
        .map(|_| ())
}

/// Sets the thread core mask (15.0.0+, system only).
pub(crate) fn set_thread_core_mask(domain: &Domain, mask: u64) -> Result<(), DispatchError> {
    // SAFETY: `mask` lives on the stack until `.send()` returns.
    unsafe {
        domain
            .dispatch(proto::SET_THREAD_CORE_MASK)
            .in_raw((&raw const mask).cast::<u8>(), size_of::<u64>())
            .send()
            .map(|_| ())
    }
}

/// Gets the thread core mask (15.0.0+, system only).
pub(crate) fn get_thread_core_mask(domain: &Domain) -> Result<u64, DispatchError> {
    let result = domain
        .dispatch(proto::GET_THREAD_CORE_MASK)
        .out_size(size_of::<u64>())
        .send()?;
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
