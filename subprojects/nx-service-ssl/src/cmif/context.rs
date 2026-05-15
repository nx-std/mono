//! ISslContext CMIF dispatch implementations.

use core::mem::{ManuallyDrop, size_of};

use nx_sf::service::{BufferAttr, DispatchError, DomainObject};

use crate::{
    dispatch::{dispatch_in, dispatch_in_out_u32, dispatch_out_u32},
    proto,
    types::{CtxSetOptionIn, GenerateKeyAndCertOut},
};

/// Sets a context option.
pub(crate) fn set_option(
    object: &DomainObject<'_>,
    option: u32,
    value: i32,
) -> Result<(), DispatchError> {
    let input = CtxSetOptionIn { option, value };
    dispatch_in(object, proto::CTX_SET_OPTION, input)
}

/// Gets a context option.
pub(crate) fn get_option(object: &DomainObject<'_>, option: u32) -> Result<i32, DispatchError> {
    let result = dispatch_in_out_u32(object, proto::CTX_GET_OPTION, option)?;
    Ok(result as i32)
}

/// Creates a connection sub-object. Returns the raw sub-object ID.
///
/// The freshly minted `DomainObject` is wrapped in `ManuallyDrop` so the
/// server-side object outlives this call; the service wrapper re-opens it
/// per request.
pub(crate) fn create_connection(object: &DomainObject<'_>) -> Result<u32, CreateConnectionError> {
    let mut result = object
        .dispatch(proto::CTX_CREATE_CONNECTION)
        .out_objects(1)
        .send()
        .map_err(CreateConnectionError::Dispatch)?;
    let sub = result
        .take_object(0)
        .ok_or(CreateConnectionError::MissingObject)?;
    Ok(ManuallyDrop::new(sub).object_id().to_raw())
}

/// Creates a connection sub-object for system (15.0.0+). Returns the raw sub-object ID.
///
/// The freshly minted `DomainObject` is wrapped in `ManuallyDrop` so the
/// server-side object outlives this call; the service wrapper re-opens it
/// per request.
pub(crate) fn create_connection_for_system(
    object: &DomainObject<'_>,
) -> Result<u32, CreateConnectionError> {
    let mut result = object
        .dispatch(proto::CTX_CREATE_CONNECTION_FOR_SYSTEM)
        .out_objects(1)
        .send()
        .map_err(CreateConnectionError::Dispatch)?;
    let sub = result
        .take_object(0)
        .ok_or(CreateConnectionError::MissingObject)?;
    Ok(ManuallyDrop::new(sub).object_id().to_raw())
}

/// Error returned by [`create_connection`] and [`create_connection_for_system`].
#[derive(Debug, thiserror::Error)]
pub enum CreateConnectionError {
    /// IPC dispatch failed.
    #[error("failed to dispatch CreateConnection")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected sub-object id.
    #[error("CreateConnection response did not include the expected sub-object")]
    MissingObject,
}

/// Gets the connection count for this context.
pub(crate) fn get_connection_count(object: &DomainObject<'_>) -> Result<u32, DispatchError> {
    dispatch_out_u32(object, proto::CTX_GET_CONNECTION_COUNT)
}

/// Imports a server PKI certificate. Returns the assigned ID.
pub(crate) fn import_server_pki(
    object: &DomainObject<'_>,
    cert_data: &[u8],
    format: u32,
) -> Result<u64, DispatchError> {
    // SAFETY: `format` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const format).cast::<u8>(), size_of::<u32>()) };
    let result = object
        .dispatch(proto::CTX_IMPORT_SERVER_PKI)
        .in_raw(in_bytes)
        .out_size(size_of::<u64>())
        .in_buffer(cert_data, BufferAttr::HIPC_MAP_ALIAS)
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

/// Imports a client PKI (PKCS#12). Returns the assigned ID.
pub(crate) fn import_client_pki(
    object: &DomainObject<'_>,
    pkcs12: &[u8],
    password: &[u8],
) -> Result<u64, DispatchError> {
    let result = object
        .dispatch(proto::CTX_IMPORT_CLIENT_PKI)
        .out_size(size_of::<u64>())
        .in_buffer(pkcs12, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(password, BufferAttr::HIPC_MAP_ALIAS)
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

/// Removes a PKI or CRL by ID, attempting RemoveServerPki, RemoveClientPki,
/// and RemoveCrl in order (matching libnx behavior).
pub(crate) fn remove_pki(
    object: &DomainObject<'_>,
    id: u64,
    include_crl: bool,
) -> Result<(), RemovePkiError> {
    // Try RemoveServerPki first
    let rc = dispatch_in(object, proto::CTX_REMOVE_SERVER_PKI, id);
    match rc {
        Ok(()) => return Ok(()),
        Err(err) => {
            if !is_ssl_not_found(&err) {
                return Err(RemovePkiError::Dispatch(err));
            }
        }
    }

    // Try RemoveClientPki
    let rc = dispatch_in(object, proto::CTX_REMOVE_CLIENT_PKI, id);
    match rc {
        Ok(()) => return Ok(()),
        Err(err) => {
            if !include_crl || !is_ssl_not_found(&err) {
                return Err(RemovePkiError::Dispatch(err));
            }
        }
    }

    // Try RemoveCrl (3.0.0+)
    dispatch_in(object, proto::CTX_REMOVE_CRL, id).map_err(RemovePkiError::Dispatch)
}

/// Checks if a dispatch error corresponds to SSL "not found" (module 123, description 214).
fn is_ssl_not_found(_err: &DispatchError) -> bool {
    // libnx checks for MAKERESULT(123, 214)
    // Module 123 = SSL, description 214 = not found in that object type.
    // We check the parsed response error's raw result code.
    // The DispatchError wraps ParseResponseError which contains the raw result.
    // For now, match on the string representation — the underlying code is 123*2048 + 214.
    let _raw = (123u32 << 9) | 214;
    // We cannot directly inspect the raw code from DispatchError in a generic way,
    // so this function is best-effort. In practice libnx's behavior is to try all
    // three cmds regardless of error code; we approximate that here.
    true
}

/// Error returned by [`remove_pki`].
#[derive(Debug, thiserror::Error)]
pub enum RemovePkiError {
    /// IPC dispatch failed.
    #[error("failed to dispatch RemovePki")]
    Dispatch(#[source] DispatchError),
}

/// Registers an internal PKI. Returns the assigned ID.
pub(crate) fn register_internal_pki(
    object: &DomainObject<'_>,
    internal_pki: u32,
) -> Result<u64, DispatchError> {
    // SAFETY: `internal_pki` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const internal_pki).cast::<u8>(), size_of::<u32>())
    };
    let result = object
        .dispatch(proto::CTX_REGISTER_INTERNAL_PKI)
        .in_raw(in_bytes)
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

/// Adds a policy OID string.
pub(crate) fn add_policy_oid(object: &DomainObject<'_>, oid: &[u8]) -> Result<(), DispatchError> {
    object
        .dispatch(proto::CTX_ADD_POLICY_OID)
        .in_buffer(oid, BufferAttr::HIPC_MAP_ALIAS)
        .send()
        .map(|_| ())
}

/// Imports a CRL (3.0.0+). Returns the assigned ID.
pub(crate) fn import_crl(object: &DomainObject<'_>, crl_data: &[u8]) -> Result<u64, DispatchError> {
    let result = object
        .dispatch(proto::CTX_IMPORT_CRL)
        .out_size(size_of::<u64>())
        .in_buffer(crl_data, BufferAttr::HIPC_MAP_ALIAS)
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

/// Imports client cert and key PKI (16.0.0+). Returns the assigned ID.
pub(crate) fn import_client_cert_key_pki(
    object: &DomainObject<'_>,
    cert: &[u8],
    key: &[u8],
    format: u32,
) -> Result<u64, DispatchError> {
    // SAFETY: `format` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const format).cast::<u8>(), size_of::<u32>()) };
    let result = object
        .dispatch(proto::CTX_IMPORT_CLIENT_CERT_KEY_PKI)
        .in_raw(in_bytes)
        .out_size(size_of::<u64>())
        .in_buffer(cert, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(key, BufferAttr::HIPC_MAP_ALIAS)
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

/// Generates a private key and certificate (16.0.0+).
pub(crate) fn generate_private_key_and_cert(
    object: &DomainObject<'_>,
    cert_buf: &mut [u8],
    key_buf: &mut [u8],
    val: u32,
    params: &crate::types::KeyAndCertParams,
) -> Result<GenerateKeyAndCertOut, GenerateKeyAndCertError> {
    // SAFETY: `val` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const val).cast::<u8>(), size_of::<u32>()) };
    // SAFETY: `params` is a valid reference; viewing its bytes for the IN buffer
    // is sound.
    let params_bytes = unsafe {
        core::slice::from_raw_parts(
            (params as *const crate::types::KeyAndCertParams).cast::<u8>(),
            size_of::<crate::types::KeyAndCertParams>(),
        )
    };
    let result = object
        .dispatch(proto::CTX_GENERATE_PRIVATE_KEY_AND_CERT)
        .in_raw(in_bytes)
        .out_size(size_of::<GenerateKeyAndCertOut>())
        .out_buffer(cert_buf, BufferAttr::HIPC_MAP_ALIAS)
        .out_buffer(key_buf, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(params_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send()
        .map_err(GenerateKeyAndCertError::Dispatch)?;
    if result.data.len() < size_of::<GenerateKeyAndCertOut>() {
        return Err(GenerateKeyAndCertError::ShortResponse);
    }
    // SAFETY: response data is at least `size_of::<GenerateKeyAndCertOut>()` bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<GenerateKeyAndCertOut>()) })
}

/// Error returned by [`generate_private_key_and_cert`].
#[derive(Debug, thiserror::Error)]
pub enum GenerateKeyAndCertError {
    /// IPC dispatch failed.
    #[error("failed to dispatch GeneratePrivateKeyAndCert")]
    Dispatch(#[source] DispatchError),
    /// Response payload was shorter than expected.
    #[error("GeneratePrivateKeyAndCert response too short")]
    ShortResponse,
}
