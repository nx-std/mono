//! ISslService CMIF dispatch implementations.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainRef,
};
use zerocopy::IntoBytes as _;

use crate::{
    proto,
    types::{
        CreateContextIn,
        DebugOptionType,
        FlushSessionCacheOptionType,
        SslVersion,
    },
};

/// Creates an SSL context. Returns the raw sub-object ID.
///
/// The close obligation is handed on rather than discharged: the caller
/// re-addresses the id through the long-lived parent domain.
pub(crate) fn create_context(
    domain: DomainRef<'_>,
    ssl_version: SslVersion,
    system: bool,
) -> Result<u32, CreateContextError> {
    let input = CreateContextIn {
        ssl_version: ssl_version.bits(),
        _pad: 0,
        pid_placeholder: 0,
    };
    let cmd_id = if system {
        proto::CREATE_CONTEXT_SYSTEM
    } else {
        proto::CREATE_CONTEXT
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let mut result = domain
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
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

impl nx_sf::error::ToResultCode for CreateContextError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Dispatch(err) => err.to_rc(),
            // As `CreateConnectionError::MissingObject`: the reply parsed and carried no object.
            Self::MissingObject => nx_sf::error::GENERIC_ERROR,
        }
    }
}

/// Gets the context count.
pub(crate) fn get_context_count(domain: DomainRef<'_>) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::GET_CONTEXT_COUNT)
        .out_size(size_of::<u32>())
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u32>())
}

/// Gets built-in certificates (pre-3.0.0, no output count).
pub(crate) fn get_certificates_legacy(
    domain: DomainRef<'_>,
    buffer: &mut [u8],
    ca_cert_ids: &[u32],
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::GET_CERTIFICATES)
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(ca_cert_ids.as_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Gets built-in certificates (3.0.0+, returns output count).
pub(crate) fn get_certificates(
    domain: DomainRef<'_>,
    buffer: &mut [u8],
    ca_cert_ids: &[u32],
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::GET_CERTIFICATES)
        .out_size(size_of::<u32>())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(ca_cert_ids.as_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u32>())
}

/// Gets the required buffer size for the given certificate IDs.
pub(crate) fn get_certificate_buf_size(
    domain: DomainRef<'_>,
    ca_cert_ids: &[u32],
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::GET_CERTIFICATE_BUF_SIZE)
        .out_size(size_of::<u32>())
        .in_buffer(ca_cert_ids.as_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u32>())
}

/// Sets the interface version (3.0.0+, internal).
pub(crate) fn set_interface_version(
    domain: DomainRef<'_>,
    version: u32,
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::SET_INTERFACE_VERSION)
        .in_raw(version.as_bytes())
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Flushes the session cache (5.0.0+).
pub(crate) fn flush_session_cache(
    domain: DomainRef<'_>,
    hostname: &[u8],
    option_type: FlushSessionCacheOptionType,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::FLUSH_SESSION_CACHE)
        .in_raw((option_type as u32).as_bytes())
        .out_size(size_of::<u32>())
        .in_buffer(hostname, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u32>())
}

/// Sets a debug option (6.0.0+).
pub(crate) fn set_debug_option(
    domain: DomainRef<'_>,
    debug_type: DebugOptionType,
    buffer: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::SET_DEBUG_OPTION)
        .in_raw((debug_type as u32).as_bytes())
        .in_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Gets a debug option (6.0.0+).
pub(crate) fn get_debug_option(
    domain: DomainRef<'_>,
    debug_type: DebugOptionType,
    buffer: &mut [u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::GET_DEBUG_OPTION)
        .in_raw((debug_type as u32).as_bytes())
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
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::SET_THREAD_CORE_MASK)
        .in_raw(mask.as_bytes())
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
    Ok(*result.value::<u64>())
}
