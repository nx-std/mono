//! ISslContext CMIF dispatch implementations.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainObjectRef,
    DomainTarget,
};
use zerocopy::IntoBytes as _;

use crate::{
    ConnectionKind,
    certificate::{
        CertificateFormat,
        GenerateKeyAndCertOut,
        InternalPki,
        KeyAndCertParams,
    },
    context::{
        ContextOption,
        CtxSetOptionIn,
    },
    dispatch::{
        dispatch_in,
        dispatch_in_out_u32,
        dispatch_out_u32,
    },
    proto,
};

/// Sets a context option.
pub(crate) fn set_option<'d>(
    object: impl DomainTarget<'d>,
    option: ContextOption,
    value: i32,
) -> Result<(), DispatchError> {
    let input = CtxSetOptionIn {
        option: option as u32,
        value,
    };
    dispatch_in(object, proto::CTX_SET_OPTION, input)
}

/// Gets a context option.
pub(crate) fn get_option<'d>(
    object: impl DomainTarget<'d>,
    option: ContextOption,
) -> Result<i32, DispatchError> {
    let result = dispatch_in_out_u32(object, proto::CTX_GET_OPTION, option as u32)?;
    Ok(result as i32)
}

/// Creates a connection sub-object of the requested kind. Returns the raw sub-object ID.
///
/// The close obligation is handed on rather than discharged: the caller
/// re-addresses the id through the long-lived parent domain.
///
/// This takes [`DomainObjectRef`] rather than [`DomainTarget`], because it adopts the object the
/// reply carries and only a domain this process owns can take on that close.
pub(crate) fn create_connection(
    object: DomainObjectRef<'_>,
    kind: ConnectionKind,
) -> Result<u32, CreateConnectionError> {
    let request_id = match kind {
        ConnectionKind::Application => proto::CTX_CREATE_CONNECTION,
        ConnectionKind::System => proto::CTX_CREATE_CONNECTION_FOR_SYSTEM,
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let mut result = object
        .dispatch(request_id)
        .out_objects(1)
        .send(&mut ipc_buf)
        .map_err(CreateConnectionError::Dispatch)?;
    let sub = result
        .take_object(0)
        .ok_or(CreateConnectionError::MissingObject)?;
    Ok(sub.into_raw_object_id())
}

/// Error returned by [`create_connection`].
#[derive(Debug, thiserror::Error)]
pub enum CreateConnectionError {
    /// IPC dispatch failed.
    #[error("failed to dispatch CreateConnection")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected sub-object id.
    #[error("CreateConnection response did not include the expected sub-object")]
    MissingObject,
}

impl nx_sf::error::ToResultCode for CreateConnectionError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Dispatch(err) => err.to_rc(),
            // No server assigned this one a code: the reply parsed, and simply carried no object
            // where the command promises one.
            Self::MissingObject => nx_sf::error::GENERIC_ERROR,
        }
    }
}

/// Gets the connection count for this context.
pub(crate) fn get_connection_count<'d>(
    object: impl DomainTarget<'d>,
) -> Result<u32, DispatchError> {
    dispatch_out_u32(object, proto::CTX_GET_CONNECTION_COUNT)
}

/// Imports a server PKI certificate. Returns the assigned ID.
pub(crate) fn import_server_pki<'d>(
    object: impl DomainTarget<'d>,
    cert_data: &[u8],
    format: CertificateFormat,
) -> Result<u64, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(proto::CTX_IMPORT_SERVER_PKI)
        .in_raw((format as u32).as_bytes())
        .out_size(size_of::<u64>())
        .in_buffer(cert_data, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u64>())
}

/// Imports a client PKI (PKCS#12). Returns the assigned ID.
pub(crate) fn import_client_pki<'d>(
    object: impl DomainTarget<'d>,
    pkcs12: &[u8],
    password: &[u8],
) -> Result<u64, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(proto::CTX_IMPORT_CLIENT_PKI)
        .out_size(size_of::<u64>())
        .in_buffer(pkcs12, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(password, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u64>())
}

/// Removes a PKI or CRL by ID, attempting RemoveServerPki, RemoveClientPki,
/// and RemoveCrl in order (matching libnx behavior).
pub(crate) fn remove_pki<'d>(
    object: impl DomainTarget<'d>,
    id: u64,
    include_crl: bool,
) -> Result<(), RemovePkiError> {
    // Try RemoveServerPki first
    let rc = dispatch_in(object, proto::CTX_REMOVE_SERVER_PKI, id);
    match rc {
        Ok(()) => return Ok(()),
        Err(err) => {
            if !is_ssl_not_found(&err) {
                return Err(RemovePkiError(err));
            }
        }
    }

    // Try RemoveClientPki
    let rc = dispatch_in(object, proto::CTX_REMOVE_CLIENT_PKI, id);
    match rc {
        Ok(()) => return Ok(()),
        Err(err) => {
            if !include_crl || !is_ssl_not_found(&err) {
                return Err(RemovePkiError(err));
            }
        }
    }

    // Try RemoveCrl (3.0.0+)
    dispatch_in(object, proto::CTX_REMOVE_CRL, id).map_err(RemovePkiError)
}

/// Whether the service answered "this object holds nothing under that id".
///
/// [`remove_pki`] tries three commands because an id names a server PKI, a client PKI or a CRL and
/// the caller does not say which. Only this one answer means "look in the next place": any other
/// failure is the answer, and moving on would replace it with whatever the next command said.
fn is_ssl_not_found(err: &DispatchError) -> bool {
    /// The SSL module's "not found in this object type", as `MAKERESULT(123, 214)` builds it.
    const SSL_NOT_FOUND: u32 = (123 & 0x1FF) | ((214 & 0x1FFF) << 9);

    matches!(
        err,
        DispatchError::ParseResponse(nx_sf::cmif::ParseError::ServiceError(SSL_NOT_FOUND))
    )
}

/// Error returned by [`remove_pki`].
#[derive(Debug, thiserror::Error)]
#[error("failed to dispatch RemovePki")]
pub struct RemovePkiError(#[source] pub DispatchError);

impl nx_sf::error::ToResultCode for RemovePkiError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        self.0.to_rc()
    }
}

/// Registers an internal PKI. Returns the assigned ID.
pub(crate) fn register_internal_pki<'d>(
    object: impl DomainTarget<'d>,
    internal_pki: InternalPki,
) -> Result<u64, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(proto::CTX_REGISTER_INTERNAL_PKI)
        .in_raw((internal_pki as u32).as_bytes())
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u64>())
}

/// Adds a policy OID string.
pub(crate) fn add_policy_oid<'d>(
    object: impl DomainTarget<'d>,
    oid: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .request(proto::CTX_ADD_POLICY_OID)
        .in_buffer(oid, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Imports a CRL (3.0.0+). Returns the assigned ID.
pub(crate) fn import_crl<'d>(
    object: impl DomainTarget<'d>,
    crl_data: &[u8],
) -> Result<u64, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(proto::CTX_IMPORT_CRL)
        .out_size(size_of::<u64>())
        .in_buffer(crl_data, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u64>())
}

/// Imports client cert and key PKI (16.0.0+). Returns the assigned ID.
pub(crate) fn import_client_cert_key_pki<'d>(
    object: impl DomainTarget<'d>,
    cert: &[u8],
    key: &[u8],
    format: CertificateFormat,
) -> Result<u64, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(proto::CTX_IMPORT_CLIENT_CERT_KEY_PKI)
        .in_raw((format as u32).as_bytes())
        .out_size(size_of::<u64>())
        .in_buffer(cert, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(key, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u64>())
}

/// Generates a private key and certificate (16.0.0+).
pub(crate) fn generate_private_key_and_cert<'d>(
    object: impl DomainTarget<'d>,
    cert_buf: &mut [u8],
    key_buf: &mut [u8],
    val: u32,
    params: &KeyAndCertParams,
) -> Result<GenerateKeyAndCertOut, GenerateKeyAndCertError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(proto::CTX_GENERATE_PRIVATE_KEY_AND_CERT)
        .in_raw(val.as_bytes())
        .out_size(size_of::<GenerateKeyAndCertOut>())
        .out_buffer(cert_buf, BufferAttr::HIPC_MAP_ALIAS)
        .out_buffer(key_buf, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(params.as_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map_err(GenerateKeyAndCertError::Dispatch)?;
    // Checked here rather than left to `value`, which panics on a short
    // payload; this command reports the truncation to the caller instead.
    if result.data.len() < size_of::<GenerateKeyAndCertOut>() {
        return Err(GenerateKeyAndCertError::ShortResponse);
    }
    Ok(*result.value::<GenerateKeyAndCertOut>())
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

impl nx_sf::error::ToResultCode for GenerateKeyAndCertError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Dispatch(err) => err.to_rc(),
            // A reply too short to hold what it claims is this crate noticing a state no server
            // reported, so it takes the code reserved for exactly that.
            Self::ShortResponse => nx_sf::error::GENERIC_ERROR,
        }
    }
}
