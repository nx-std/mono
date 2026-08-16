//! The C boundary's view of an `ISslContext`.

use nx_sf::service::{
    DispatchError,
    ForeignDomainObject,
};

use crate::{
    cmif::{
        self,
        GenerateKeyAndCertError,
        RemovePkiError,
    },
    types::{
        CertificateFormat,
        ContextOption,
        InternalPki,
        KeyAndCertParams,
    },
};

/// An `ISslContext` a C caller owns.
///
/// The borrowed counterpart of [`SslContext`](crate::SslContext), reached the same way
/// [`ForeignSslConnection`](super::ForeignSslConnection) is: through
/// [`nx_sf::ffi::Service::as_domain_object`], off a struct the C caller holds the close for.
///
/// # It cannot create a connection
///
/// [`SslContext::create_connection`](crate::SslContext::create_connection) has no counterpart
/// here, because creating one means adopting the object the reply carries, and a
/// [`ForeignDomainObject`] cannot: the close would be owed by whoever owns the domain, not by the
/// view that asked. The service that owns the domain does it instead, through
/// [`SslService::create_connection_under`](crate::SslService::create_connection_under).
#[derive(Debug, Clone, Copy)]
pub struct ForeignSslContext<'a> {
    object: ForeignDomainObject<'a>,
}

impl<'a> ForeignSslContext<'a> {
    /// Views the `ISslContext` object `object` addresses.
    #[inline]
    pub fn new(object: ForeignDomainObject<'a>) -> Self {
        Self { object }
    }

    /// Sets a context option.
    pub fn set_option(&self, option: ContextOption, value: i32) -> Result<(), DispatchError> {
        cmif::ctx_set_option(self.object, option, value)
    }

    /// Gets a context option.
    pub fn get_option(&self, option: ContextOption) -> Result<i32, DispatchError> {
        cmif::ctx_get_option(self.object, option)
    }

    /// Gets the connection count for this context.
    pub fn get_connection_count(&self) -> Result<u32, DispatchError> {
        cmif::get_connection_count(self.object)
    }

    /// Imports server PKI certificate(s). Returns the assigned ID.
    pub fn import_server_pki(
        &self,
        cert_data: &[u8],
        format: CertificateFormat,
    ) -> Result<u64, DispatchError> {
        cmif::import_server_pki(self.object, cert_data, format)
    }

    /// Imports a client PKI (PKCS#12). Returns the assigned ID.
    pub fn import_client_pki(&self, pkcs12: &[u8], password: &[u8]) -> Result<u64, DispatchError> {
        cmif::import_client_pki(self.object, pkcs12, password)
    }

    /// Removes a PKI or CRL by ID.
    pub fn remove_pki(&self, id: u64, include_crl: bool) -> Result<(), RemovePkiError> {
        cmif::remove_pki(self.object, id, include_crl)
    }

    /// Registers an internal PKI. Returns the assigned ID.
    pub fn register_internal_pki(&self, pki: InternalPki) -> Result<u64, DispatchError> {
        cmif::register_internal_pki(self.object, pki)
    }

    /// Adds a policy OID string.
    pub fn add_policy_oid(&self, oid: &[u8]) -> Result<(), DispatchError> {
        cmif::add_policy_oid(self.object, oid)
    }

    /// Imports a CRL (3.0.0+). Returns the assigned ID.
    pub fn import_crl(&self, crl_data: &[u8]) -> Result<u64, DispatchError> {
        cmif::import_crl(self.object, crl_data)
    }

    /// Imports client cert and key PKI (16.0.0+). Returns the assigned ID.
    pub fn import_client_cert_key_pki(
        &self,
        cert: &[u8],
        key: &[u8],
        format: CertificateFormat,
    ) -> Result<u64, DispatchError> {
        cmif::import_client_cert_key_pki(self.object, cert, key, format)
    }

    /// Generates a private key and certificate (16.0.0+).
    ///
    /// Returns `(cert_size, key_size)`: the actual sizes written to the output buffers.
    pub fn generate_private_key_and_cert(
        &self,
        cert_buf: &mut [u8],
        key_buf: &mut [u8],
        params: &KeyAndCertParams,
    ) -> Result<(u32, u32), GenerateKeyAndCertError> {
        let out = cmif::generate_private_key_and_cert(self.object, cert_buf, key_buf, 1, params)?;
        Ok((out.cert_size, out.key_size))
    }
}
