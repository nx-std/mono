//! Security Processor Liaison (`spl`) service implementation.
//!
//! Provides access to the hardware security processor for key management,
//! AES operations, RSA operations, and device configuration queries.
//!
//! ## Service Variants
//!
//! Six service endpoints are available, split at \[4.0.0\]:
//!
//! - **`spl:`** — General interface. Connected via [`connect_general_cmif`].
//! - **`spl:mig`** — Crypto interface (4.0.0+). Connected via [`connect_crypto_cmif`].
//! - **`spl:ssl`** — SSL interface (4.0.0+). Connected via [`connect_ssl_cmif`].
//! - **`spl:es`** — ES interface (4.0.0+). Connected via [`connect_es_cmif`].
//! - **`spl:fs`** — FS interface (4.0.0+). Connected via [`connect_fs_cmif`].
//! - **`spl:manu`** — Manufacturing interface (4.0.0+). Connected via [`connect_manu_cmif`].
//!
//! On pre-4.0.0 all functionality is accessed through `spl:` (the general
//! interface contains all commands). On 4.0.0+ the service was split into
//! specialised endpoints. The caller selects which variant to connect to
//! based on the system version and required functionality.
//!
//! ## Hosversion variants
//!
//! Commands with different wire layouts across versions are exposed as paired
//! `_legacy` (pre-5.0.0) and non-suffixed (5.0.0+) variants. Commands that
//! only exist on newer firmware are exposed unconditionally.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{DispatchError, Session};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::GetSecurityEngineEventError,
    proto::{
        CRYPTO_SERVICE_NAME, ES_SERVICE_NAME, FS_SERVICE_NAME, GENERAL_SERVICE_NAME,
        MANU_SERVICE_NAME, SSL_SERVICE_NAME,
    },
    types::{RSA_BUFFER_SIZE, RsaKeyVersion, SHA256_HASH_SIZE, SplConfigItem, SplKey},
};

// ---------------------------------------------------------------------------
// General service (spl:)
// ---------------------------------------------------------------------------

/// Connected `spl:` (IGeneralInterface) service wrapper.
pub struct SplGeneralService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for SplGeneralService {}
unsafe impl Sync for SplGeneralService {}

impl SplGeneralService {
    /// Gets a configuration value from the security processor (cmd 0).
    #[inline]
    pub fn get_config(&self, config_item: SplConfigItem) -> Result<u64, DispatchError> {
        cmif::get_config(&self.0, config_item as u32)
    }

    /// Performs a user-mode modular exponentiation (cmd 1).
    #[inline]
    pub fn user_exp_mod(
        &self,
        input: &[u8; RSA_BUFFER_SIZE],
        modulus: &[u8; RSA_BUFFER_SIZE],
        exp: &[u8],
        dst: &mut [u8; RSA_BUFFER_SIZE],
    ) -> Result<(), DispatchError> {
        cmif::user_exp_mod(&self.0, input, modulus, exp, dst)
    }

    /// Sets a configuration value on the security processor (cmd 5).
    #[inline]
    pub fn set_config(&self, config_item: SplConfigItem, value: u64) -> Result<(), DispatchError> {
        cmif::set_config(&self.0, config_item as u32, value)
    }

    /// Gets random bytes from the security processor (cmd 7).
    #[inline]
    pub fn get_random_bytes(&self, out: &mut [u8]) -> Result<(), DispatchError> {
        cmif::get_random_bytes(&self.0, out)
    }

    /// Queries whether the device is a development unit (cmd 11).
    #[inline]
    pub fn is_development(&self) -> Result<bool, DispatchError> {
        cmif::is_development(&self.0)
    }

    /// Sets the boot reason (cmd 24, 3.0.0+).
    #[inline]
    pub fn set_boot_reason(&self, value: u32) -> Result<(), DispatchError> {
        cmif::set_boot_reason(&self.0, value)
    }

    /// Gets the boot reason (cmd 25, 3.0.0+).
    #[inline]
    pub fn get_boot_reason(&self) -> Result<u32, DispatchError> {
        cmif::get_boot_reason(&self.0)
    }
}

/// Connects to the general SPL service (`spl:`) using CMIF.
pub fn connect_general_cmif(sm: &SmService) -> Result<SplGeneralService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(GENERAL_SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    Ok(SplGeneralService(Session::new(handle, 0)))
}

// ---------------------------------------------------------------------------
// Crypto service (spl:mig)
// ---------------------------------------------------------------------------

/// Connected `spl:mig` (ICryptoInterface) service wrapper.
///
/// Includes all IGeneralInterface commands plus crypto-specific commands.
pub struct SplCryptoService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for SplCryptoService {}
unsafe impl Sync for SplCryptoService {}

impl SplCryptoService {
    // --- IGeneralInterface commands ---

    /// Gets a configuration value from the security processor (cmd 0).
    #[inline]
    pub fn get_config(&self, config_item: SplConfigItem) -> Result<u64, DispatchError> {
        cmif::get_config(&self.0, config_item as u32)
    }

    /// Performs a user-mode modular exponentiation (cmd 1).
    #[inline]
    pub fn user_exp_mod(
        &self,
        input: &[u8; RSA_BUFFER_SIZE],
        modulus: &[u8; RSA_BUFFER_SIZE],
        exp: &[u8],
        dst: &mut [u8; RSA_BUFFER_SIZE],
    ) -> Result<(), DispatchError> {
        cmif::user_exp_mod(&self.0, input, modulus, exp, dst)
    }

    /// Sets a configuration value on the security processor (cmd 5).
    #[inline]
    pub fn set_config(&self, config_item: SplConfigItem, value: u64) -> Result<(), DispatchError> {
        cmif::set_config(&self.0, config_item as u32, value)
    }

    /// Gets random bytes from the security processor (cmd 7).
    #[inline]
    pub fn get_random_bytes(&self, out: &mut [u8]) -> Result<(), DispatchError> {
        cmif::get_random_bytes(&self.0, out)
    }

    /// Queries whether the device is a development unit (cmd 11).
    #[inline]
    pub fn is_development(&self) -> Result<bool, DispatchError> {
        cmif::is_development(&self.0)
    }

    /// Sets the boot reason (cmd 24, 3.0.0+).
    #[inline]
    pub fn set_boot_reason(&self, value: u32) -> Result<(), DispatchError> {
        cmif::set_boot_reason(&self.0, value)
    }

    /// Gets the boot reason (cmd 25, 3.0.0+).
    #[inline]
    pub fn get_boot_reason(&self) -> Result<u32, DispatchError> {
        cmif::get_boot_reason(&self.0)
    }

    // --- ICryptoInterface commands ---

    /// Generates an AES KEK from a wrapped KEK (cmd 2).
    #[inline]
    pub fn generate_aes_kek(
        &self,
        wrapped_kek: &SplKey,
        key_generation: u32,
        option: u32,
    ) -> Result<SplKey, DispatchError> {
        cmif::generate_aes_kek(&self.0, wrapped_kek, key_generation, option)
    }

    /// Loads an AES key into a keyslot (cmd 3).
    #[inline]
    pub fn load_aes_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        keyslot: u32,
    ) -> Result<(), DispatchError> {
        cmif::load_aes_key(&self.0, sealed_kek, wrapped_key, keyslot)
    }

    /// Generates a sealed AES key (cmd 4).
    #[inline]
    pub fn generate_aes_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
    ) -> Result<SplKey, DispatchError> {
        cmif::generate_aes_key(&self.0, sealed_kek, wrapped_key)
    }

    /// Decrypts a wrapped AES key (cmd 14).
    #[inline]
    pub fn decrypt_aes_key(
        &self,
        wrapped_key: &SplKey,
        key_generation: u32,
        option: u32,
    ) -> Result<SplKey, DispatchError> {
        cmif::decrypt_aes_key(&self.0, wrapped_key, key_generation, option)
    }

    /// Encrypts/decrypts data using AES-CTR mode (cmd 15).
    #[inline]
    pub fn crypt_aes_ctr(
        &self,
        input: &[u8],
        output: &mut [u8],
        keyslot: u32,
        ctr: &SplKey,
    ) -> Result<(), DispatchError> {
        cmif::crypt_aes_ctr(&self.0, input, output, keyslot, ctr)
    }

    /// Computes AES-CMAC over input data (cmd 16).
    #[inline]
    pub fn compute_cmac(&self, input: &[u8], keyslot: u32) -> Result<SplKey, DispatchError> {
        cmif::compute_cmac(&self.0, input, keyslot)
    }

    /// Locks an AES engine keyslot (cmd 21, 2.0.0+).
    #[inline]
    pub fn lock_aes_engine(&self) -> Result<u32, DispatchError> {
        cmif::lock_aes_engine(&self.0)
    }

    /// Unlocks an AES engine keyslot (cmd 22, 2.0.0+).
    #[inline]
    pub fn unlock_aes_engine(&self, keyslot: u32) -> Result<(), DispatchError> {
        cmif::unlock_aes_engine(&self.0, keyslot)
    }

    /// Gets the security engine event handle (cmd 23, 2.0.0+).
    #[inline]
    pub fn get_security_engine_event(&self) -> Result<u32, GetSecurityEngineEventError> {
        cmif::get_security_engine_event(&self.0)
    }
}

/// Connects to the crypto SPL service (`spl:mig`, 4.0.0+) using CMIF.
pub fn connect_crypto_cmif(sm: &SmService) -> Result<SplCryptoService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(CRYPTO_SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    Ok(SplCryptoService(Session::new(handle, 0)))
}

// ---------------------------------------------------------------------------
// SSL service (spl:ssl)
// ---------------------------------------------------------------------------

/// Connected `spl:ssl` (ISslInterface) service wrapper.
///
/// Includes IGeneralInterface + ICryptoInterface + IRsaService + SSL-specific
/// commands.
pub struct SplSslService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for SplSslService {}
unsafe impl Sync for SplSslService {}

impl SplSslService {
    // --- IGeneralInterface commands ---

    /// Gets a configuration value from the security processor (cmd 0).
    #[inline]
    pub fn get_config(&self, config_item: SplConfigItem) -> Result<u64, DispatchError> {
        cmif::get_config(&self.0, config_item as u32)
    }

    /// Gets random bytes from the security processor (cmd 7).
    #[inline]
    pub fn get_random_bytes(&self, out: &mut [u8]) -> Result<(), DispatchError> {
        cmif::get_random_bytes(&self.0, out)
    }

    /// Queries whether the device is a development unit (cmd 11).
    #[inline]
    pub fn is_development(&self) -> Result<bool, DispatchError> {
        cmif::is_development(&self.0)
    }

    // --- ICryptoInterface commands ---

    /// Generates an AES KEK from a wrapped KEK (cmd 2).
    #[inline]
    pub fn generate_aes_kek(
        &self,
        wrapped_kek: &SplKey,
        key_generation: u32,
        option: u32,
    ) -> Result<SplKey, DispatchError> {
        cmif::generate_aes_kek(&self.0, wrapped_kek, key_generation, option)
    }

    /// Loads an AES key into a keyslot (cmd 3).
    #[inline]
    pub fn load_aes_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        keyslot: u32,
    ) -> Result<(), DispatchError> {
        cmif::load_aes_key(&self.0, sealed_kek, wrapped_key, keyslot)
    }

    /// Generates a sealed AES key (cmd 4).
    #[inline]
    pub fn generate_aes_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
    ) -> Result<SplKey, DispatchError> {
        cmif::generate_aes_key(&self.0, sealed_kek, wrapped_key)
    }

    /// Decrypts a wrapped AES key (cmd 14).
    #[inline]
    pub fn decrypt_aes_key(
        &self,
        wrapped_key: &SplKey,
        key_generation: u32,
        option: u32,
    ) -> Result<SplKey, DispatchError> {
        cmif::decrypt_aes_key(&self.0, wrapped_key, key_generation, option)
    }

    /// Encrypts/decrypts data using AES-CTR mode (cmd 15).
    #[inline]
    pub fn crypt_aes_ctr(
        &self,
        input: &[u8],
        output: &mut [u8],
        keyslot: u32,
        ctr: &SplKey,
    ) -> Result<(), DispatchError> {
        cmif::crypt_aes_ctr(&self.0, input, output, keyslot, ctr)
    }

    /// Computes AES-CMAC over input data (cmd 16).
    #[inline]
    pub fn compute_cmac(&self, input: &[u8], keyslot: u32) -> Result<SplKey, DispatchError> {
        cmif::compute_cmac(&self.0, input, keyslot)
    }

    /// Locks an AES engine keyslot (cmd 21, 2.0.0+).
    #[inline]
    pub fn lock_aes_engine(&self) -> Result<u32, DispatchError> {
        cmif::lock_aes_engine(&self.0)
    }

    /// Unlocks an AES engine keyslot (cmd 22, 2.0.0+).
    #[inline]
    pub fn unlock_aes_engine(&self, keyslot: u32) -> Result<(), DispatchError> {
        cmif::unlock_aes_engine(&self.0, keyslot)
    }

    /// Gets the security engine event handle (cmd 23, 2.0.0+).
    #[inline]
    pub fn get_security_engine_event(&self) -> Result<u32, GetSecurityEngineEventError> {
        cmif::get_security_engine_event(&self.0)
    }

    // --- IRsaService commands ---

    /// Decrypts an RSA private key, legacy wire format (pre-5.0.0, cmd 13).
    #[inline]
    pub fn decrypt_rsa_private_key_legacy(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        version: RsaKeyVersion,
        wrapped_rsa_key: &[u8],
        dst: &mut [u8],
    ) -> Result<(), DispatchError> {
        cmif::decrypt_rsa_private_key_legacy(
            &self.0,
            sealed_kek,
            wrapped_key,
            version,
            wrapped_rsa_key,
            dst,
        )
    }

    /// Decrypts an RSA private key (5.0.0+, cmd 13).
    #[inline]
    pub fn decrypt_rsa_private_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        wrapped_rsa_key: &[u8],
        dst: &mut [u8],
    ) -> Result<(), DispatchError> {
        cmif::decrypt_rsa_private_key(&self.0, sealed_kek, wrapped_key, wrapped_rsa_key, dst)
    }

    // --- ISslInterface commands ---

    /// Loads a secure exponent-modulus key for SSL (cmd 26, 5.0.0+).
    #[inline]
    pub fn load_secure_exp_mod_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        wrapped_rsa_key: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::ssl_load_secure_exp_mod_key(&self.0, sealed_kek, wrapped_key, wrapped_rsa_key)
    }

    /// Performs a secure modular exponentiation for SSL (cmd 27, 5.0.0+).
    #[inline]
    pub fn secure_exp_mod(
        &self,
        input: &[u8; RSA_BUFFER_SIZE],
        modulus: &[u8; RSA_BUFFER_SIZE],
        dst: &mut [u8; RSA_BUFFER_SIZE],
    ) -> Result<(), DispatchError> {
        cmif::ssl_secure_exp_mod(&self.0, input, modulus, dst)
    }
}

/// Connects to the SSL SPL service (`spl:ssl`, 4.0.0+) using CMIF.
pub fn connect_ssl_cmif(sm: &SmService) -> Result<SplSslService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SSL_SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    Ok(SplSslService(Session::new(handle, 0)))
}

// ---------------------------------------------------------------------------
// ES service (spl:es)
// ---------------------------------------------------------------------------

/// Connected `spl:es` (IEsInterface) service wrapper.
///
/// Includes IGeneralInterface + ICryptoInterface + IRsaService + ES-specific
/// commands.
pub struct SplEsService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for SplEsService {}
unsafe impl Sync for SplEsService {}

impl SplEsService {
    // --- IGeneralInterface commands ---

    /// Gets a configuration value from the security processor (cmd 0).
    #[inline]
    pub fn get_config(&self, config_item: SplConfigItem) -> Result<u64, DispatchError> {
        cmif::get_config(&self.0, config_item as u32)
    }

    /// Gets random bytes from the security processor (cmd 7).
    #[inline]
    pub fn get_random_bytes(&self, out: &mut [u8]) -> Result<(), DispatchError> {
        cmif::get_random_bytes(&self.0, out)
    }

    /// Queries whether the device is a development unit (cmd 11).
    #[inline]
    pub fn is_development(&self) -> Result<bool, DispatchError> {
        cmif::is_development(&self.0)
    }

    // --- ICryptoInterface commands ---

    /// Generates an AES KEK from a wrapped KEK (cmd 2).
    #[inline]
    pub fn generate_aes_kek(
        &self,
        wrapped_kek: &SplKey,
        key_generation: u32,
        option: u32,
    ) -> Result<SplKey, DispatchError> {
        cmif::generate_aes_kek(&self.0, wrapped_kek, key_generation, option)
    }

    /// Loads an AES key into a keyslot (cmd 3).
    #[inline]
    pub fn load_aes_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        keyslot: u32,
    ) -> Result<(), DispatchError> {
        cmif::load_aes_key(&self.0, sealed_kek, wrapped_key, keyslot)
    }

    /// Generates a sealed AES key (cmd 4).
    #[inline]
    pub fn generate_aes_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
    ) -> Result<SplKey, DispatchError> {
        cmif::generate_aes_key(&self.0, sealed_kek, wrapped_key)
    }

    /// Decrypts a wrapped AES key (cmd 14).
    #[inline]
    pub fn decrypt_aes_key(
        &self,
        wrapped_key: &SplKey,
        key_generation: u32,
        option: u32,
    ) -> Result<SplKey, DispatchError> {
        cmif::decrypt_aes_key(&self.0, wrapped_key, key_generation, option)
    }

    /// Encrypts/decrypts data using AES-CTR mode (cmd 15).
    #[inline]
    pub fn crypt_aes_ctr(
        &self,
        input: &[u8],
        output: &mut [u8],
        keyslot: u32,
        ctr: &SplKey,
    ) -> Result<(), DispatchError> {
        cmif::crypt_aes_ctr(&self.0, input, output, keyslot, ctr)
    }

    /// Computes AES-CMAC over input data (cmd 16).
    #[inline]
    pub fn compute_cmac(&self, input: &[u8], keyslot: u32) -> Result<SplKey, DispatchError> {
        cmif::compute_cmac(&self.0, input, keyslot)
    }

    /// Locks an AES engine keyslot (cmd 21, 2.0.0+).
    #[inline]
    pub fn lock_aes_engine(&self) -> Result<u32, DispatchError> {
        cmif::lock_aes_engine(&self.0)
    }

    /// Unlocks an AES engine keyslot (cmd 22, 2.0.0+).
    #[inline]
    pub fn unlock_aes_engine(&self, keyslot: u32) -> Result<(), DispatchError> {
        cmif::unlock_aes_engine(&self.0, keyslot)
    }

    /// Gets the security engine event handle (cmd 23, 2.0.0+).
    #[inline]
    pub fn get_security_engine_event(&self) -> Result<u32, GetSecurityEngineEventError> {
        cmif::get_security_engine_event(&self.0)
    }

    // --- IRsaService commands ---

    /// Decrypts an RSA private key, legacy wire format (pre-5.0.0, cmd 13).
    #[inline]
    pub fn decrypt_rsa_private_key_legacy(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        version: RsaKeyVersion,
        wrapped_rsa_key: &[u8],
        dst: &mut [u8],
    ) -> Result<(), DispatchError> {
        cmif::decrypt_rsa_private_key_legacy(
            &self.0,
            sealed_kek,
            wrapped_key,
            version,
            wrapped_rsa_key,
            dst,
        )
    }

    /// Decrypts an RSA private key (5.0.0+, cmd 13).
    #[inline]
    pub fn decrypt_rsa_private_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        wrapped_rsa_key: &[u8],
        dst: &mut [u8],
    ) -> Result<(), DispatchError> {
        cmif::decrypt_rsa_private_key(&self.0, sealed_kek, wrapped_key, wrapped_rsa_key, dst)
    }

    // --- IEsInterface commands ---

    /// Loads an RSA-OAEP key, legacy wire format (pre-5.0.0, cmd 17).
    #[inline]
    pub fn load_rsa_oaep_key_legacy(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        wrapped_rsa_key: &[u8],
        version: RsaKeyVersion,
    ) -> Result<(), DispatchError> {
        cmif::es_load_rsa_oaep_key_legacy(
            &self.0,
            sealed_kek,
            wrapped_key,
            wrapped_rsa_key,
            version,
        )
    }

    /// Loads an RSA-OAEP key (5.0.0+, cmd 17).
    #[inline]
    pub fn load_rsa_oaep_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        wrapped_rsa_key: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::es_load_rsa_oaep_key(&self.0, sealed_kek, wrapped_key, wrapped_rsa_key)
    }

    /// Unwraps an RSA-OAEP-wrapped titlekey (cmd 18).
    #[inline]
    pub fn unwrap_rsa_oaep_wrapped_titlekey(
        &self,
        rsa_wrapped_titlekey: &[u8; RSA_BUFFER_SIZE],
        modulus: &[u8; RSA_BUFFER_SIZE],
        label_hash: &[u8],
        key_generation: u32,
    ) -> Result<SplKey, DispatchError> {
        cmif::es_unwrap_rsa_oaep_wrapped_titlekey(
            &self.0,
            rsa_wrapped_titlekey,
            modulus,
            label_hash,
            key_generation,
        )
    }

    /// Unwraps an AES-wrapped titlekey (cmd 20, 2.0.0+).
    #[inline]
    pub fn unwrap_aes_wrapped_titlekey(
        &self,
        aes_wrapped_titlekey: &SplKey,
        key_generation: u32,
    ) -> Result<SplKey, DispatchError> {
        cmif::es_unwrap_aes_wrapped_titlekey(&self.0, aes_wrapped_titlekey, key_generation)
    }

    /// Loads a secure exponent-modulus key for ES (cmd 28, 5.0.0+).
    #[inline]
    pub fn load_secure_exp_mod_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        wrapped_rsa_key: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::es_load_secure_exp_mod_key(&self.0, sealed_kek, wrapped_key, wrapped_rsa_key)
    }

    /// Performs a secure modular exponentiation for ES (cmd 29, 5.0.0+).
    #[inline]
    pub fn secure_exp_mod(
        &self,
        input: &[u8; RSA_BUFFER_SIZE],
        modulus: &[u8; RSA_BUFFER_SIZE],
        dst: &mut [u8; RSA_BUFFER_SIZE],
    ) -> Result<(), DispatchError> {
        cmif::es_secure_exp_mod(&self.0, input, modulus, dst)
    }

    /// Unwraps an e-license key (cmd 31, 6.0.0+).
    #[inline]
    pub fn unwrap_elicense_key(
        &self,
        rsa_wrapped_key: &[u8; RSA_BUFFER_SIZE],
        modulus: &[u8; RSA_BUFFER_SIZE],
        label_hash: &[u8],
        key_generation: u32,
    ) -> Result<SplKey, DispatchError> {
        cmif::es_unwrap_elicense_key(
            &self.0,
            rsa_wrapped_key,
            modulus,
            label_hash,
            key_generation,
        )
    }

    /// Loads an e-license key into a keyslot (cmd 32, 6.0.0+).
    #[inline]
    pub fn load_elicense_key(
        &self,
        sealed_key: &SplKey,
        keyslot: u32,
    ) -> Result<(), DispatchError> {
        cmif::es_load_elicense_key(&self.0, sealed_key, keyslot)
    }
}

/// Connects to the ES SPL service (`spl:es`, 4.0.0+) using CMIF.
pub fn connect_es_cmif(sm: &SmService) -> Result<SplEsService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(ES_SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    Ok(SplEsService(Session::new(handle, 0)))
}

// ---------------------------------------------------------------------------
// FS service (spl:fs)
// ---------------------------------------------------------------------------

/// Connected `spl:fs` (IFsInterface) service wrapper.
///
/// Includes IGeneralInterface + ICryptoInterface + IRsaService + FS-specific
/// commands.
pub struct SplFsService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for SplFsService {}
unsafe impl Sync for SplFsService {}

impl SplFsService {
    // --- IGeneralInterface commands ---

    /// Gets a configuration value from the security processor (cmd 0).
    #[inline]
    pub fn get_config(&self, config_item: SplConfigItem) -> Result<u64, DispatchError> {
        cmif::get_config(&self.0, config_item as u32)
    }

    /// Gets random bytes from the security processor (cmd 7).
    #[inline]
    pub fn get_random_bytes(&self, out: &mut [u8]) -> Result<(), DispatchError> {
        cmif::get_random_bytes(&self.0, out)
    }

    /// Queries whether the device is a development unit (cmd 11).
    #[inline]
    pub fn is_development(&self) -> Result<bool, DispatchError> {
        cmif::is_development(&self.0)
    }

    // --- ICryptoInterface commands ---

    /// Generates an AES KEK from a wrapped KEK (cmd 2).
    #[inline]
    pub fn generate_aes_kek(
        &self,
        wrapped_kek: &SplKey,
        key_generation: u32,
        option: u32,
    ) -> Result<SplKey, DispatchError> {
        cmif::generate_aes_kek(&self.0, wrapped_kek, key_generation, option)
    }

    /// Loads an AES key into a keyslot (cmd 3).
    #[inline]
    pub fn load_aes_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        keyslot: u32,
    ) -> Result<(), DispatchError> {
        cmif::load_aes_key(&self.0, sealed_kek, wrapped_key, keyslot)
    }

    /// Generates a sealed AES key (cmd 4).
    #[inline]
    pub fn generate_aes_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
    ) -> Result<SplKey, DispatchError> {
        cmif::generate_aes_key(&self.0, sealed_kek, wrapped_key)
    }

    /// Decrypts a wrapped AES key (cmd 14).
    #[inline]
    pub fn decrypt_aes_key(
        &self,
        wrapped_key: &SplKey,
        key_generation: u32,
        option: u32,
    ) -> Result<SplKey, DispatchError> {
        cmif::decrypt_aes_key(&self.0, wrapped_key, key_generation, option)
    }

    /// Encrypts/decrypts data using AES-CTR mode (cmd 15).
    #[inline]
    pub fn crypt_aes_ctr(
        &self,
        input: &[u8],
        output: &mut [u8],
        keyslot: u32,
        ctr: &SplKey,
    ) -> Result<(), DispatchError> {
        cmif::crypt_aes_ctr(&self.0, input, output, keyslot, ctr)
    }

    /// Computes AES-CMAC over input data (cmd 16).
    #[inline]
    pub fn compute_cmac(&self, input: &[u8], keyslot: u32) -> Result<SplKey, DispatchError> {
        cmif::compute_cmac(&self.0, input, keyslot)
    }

    /// Locks an AES engine keyslot (cmd 21, 2.0.0+).
    #[inline]
    pub fn lock_aes_engine(&self) -> Result<u32, DispatchError> {
        cmif::lock_aes_engine(&self.0)
    }

    /// Unlocks an AES engine keyslot (cmd 22, 2.0.0+).
    #[inline]
    pub fn unlock_aes_engine(&self, keyslot: u32) -> Result<(), DispatchError> {
        cmif::unlock_aes_engine(&self.0, keyslot)
    }

    /// Gets the security engine event handle (cmd 23, 2.0.0+).
    #[inline]
    pub fn get_security_engine_event(&self) -> Result<u32, GetSecurityEngineEventError> {
        cmif::get_security_engine_event(&self.0)
    }

    // --- IRsaService commands ---

    /// Decrypts an RSA private key, legacy wire format (pre-5.0.0, cmd 13).
    #[inline]
    pub fn decrypt_rsa_private_key_legacy(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        version: RsaKeyVersion,
        wrapped_rsa_key: &[u8],
        dst: &mut [u8],
    ) -> Result<(), DispatchError> {
        cmif::decrypt_rsa_private_key_legacy(
            &self.0,
            sealed_kek,
            wrapped_key,
            version,
            wrapped_rsa_key,
            dst,
        )
    }

    /// Decrypts an RSA private key (5.0.0+, cmd 13).
    #[inline]
    pub fn decrypt_rsa_private_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        wrapped_rsa_key: &[u8],
        dst: &mut [u8],
    ) -> Result<(), DispatchError> {
        cmif::decrypt_rsa_private_key(&self.0, sealed_kek, wrapped_key, wrapped_rsa_key, dst)
    }

    // --- IFsInterface commands ---

    /// Loads a secure exponent-modulus key for FS, legacy (pre-5.0.0, cmd 9).
    #[inline]
    pub fn load_secure_exp_mod_key_legacy(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        wrapped_rsa_key: &[u8],
        version: RsaKeyVersion,
    ) -> Result<(), DispatchError> {
        cmif::fs_load_secure_exp_mod_key_legacy(
            &self.0,
            sealed_kek,
            wrapped_key,
            wrapped_rsa_key,
            version,
        )
    }

    /// Loads a secure exponent-modulus key for FS (5.0.0+, cmd 9).
    #[inline]
    pub fn load_secure_exp_mod_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        wrapped_rsa_key: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::fs_load_secure_exp_mod_key(&self.0, sealed_kek, wrapped_key, wrapped_rsa_key)
    }

    /// Performs a secure modular exponentiation for FS (cmd 10).
    #[inline]
    pub fn secure_exp_mod(
        &self,
        input: &[u8; RSA_BUFFER_SIZE],
        modulus: &[u8; RSA_BUFFER_SIZE],
        dst: &mut [u8; RSA_BUFFER_SIZE],
    ) -> Result<(), DispatchError> {
        cmif::fs_secure_exp_mod(&self.0, input, modulus, dst)
    }

    /// Generates a specific AES key for FS (cmd 12).
    #[inline]
    pub fn generate_specific_aes_key(
        &self,
        wrapped_key: &SplKey,
        key_generation: u32,
        option: u32,
    ) -> Result<SplKey, DispatchError> {
        cmif::fs_generate_specific_aes_key(&self.0, wrapped_key, key_generation, option)
    }

    /// Loads a titlekey into a keyslot (cmd 19).
    #[inline]
    pub fn load_titlekey(
        &self,
        sealed_titlekey: &SplKey,
        keyslot: u32,
    ) -> Result<(), DispatchError> {
        cmif::fs_load_titlekey(&self.0, sealed_titlekey, keyslot)
    }

    /// Gets the package2 hash (cmd 31, 5.0.0+).
    #[inline]
    pub fn get_package2_hash(
        &self,
        out_hash: &mut [u8; SHA256_HASH_SIZE],
    ) -> Result<(), DispatchError> {
        cmif::fs_get_package2_hash(&self.0, out_hash)
    }
}

/// Connects to the FS SPL service (`spl:fs`, 4.0.0+) using CMIF.
pub fn connect_fs_cmif(sm: &SmService) -> Result<SplFsService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(FS_SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    Ok(SplFsService(Session::new(handle, 0)))
}

// ---------------------------------------------------------------------------
// Manufacturing service (spl:manu)
// ---------------------------------------------------------------------------

/// Connected `spl:manu` (IManuInterface) service wrapper.
///
/// Includes IGeneralInterface + ICryptoInterface + manufacturing-specific
/// commands.
pub struct SplManuService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for SplManuService {}
unsafe impl Sync for SplManuService {}

impl SplManuService {
    // --- IGeneralInterface commands ---

    /// Gets a configuration value from the security processor (cmd 0).
    #[inline]
    pub fn get_config(&self, config_item: SplConfigItem) -> Result<u64, DispatchError> {
        cmif::get_config(&self.0, config_item as u32)
    }

    /// Gets random bytes from the security processor (cmd 7).
    #[inline]
    pub fn get_random_bytes(&self, out: &mut [u8]) -> Result<(), DispatchError> {
        cmif::get_random_bytes(&self.0, out)
    }

    /// Queries whether the device is a development unit (cmd 11).
    #[inline]
    pub fn is_development(&self) -> Result<bool, DispatchError> {
        cmif::is_development(&self.0)
    }

    // --- ICryptoInterface commands ---

    /// Generates an AES KEK from a wrapped KEK (cmd 2).
    #[inline]
    pub fn generate_aes_kek(
        &self,
        wrapped_kek: &SplKey,
        key_generation: u32,
        option: u32,
    ) -> Result<SplKey, DispatchError> {
        cmif::generate_aes_kek(&self.0, wrapped_kek, key_generation, option)
    }

    /// Loads an AES key into a keyslot (cmd 3).
    #[inline]
    pub fn load_aes_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
        keyslot: u32,
    ) -> Result<(), DispatchError> {
        cmif::load_aes_key(&self.0, sealed_kek, wrapped_key, keyslot)
    }

    /// Generates a sealed AES key (cmd 4).
    #[inline]
    pub fn generate_aes_key(
        &self,
        sealed_kek: &SplKey,
        wrapped_key: &SplKey,
    ) -> Result<SplKey, DispatchError> {
        cmif::generate_aes_key(&self.0, sealed_kek, wrapped_key)
    }

    /// Decrypts a wrapped AES key (cmd 14).
    #[inline]
    pub fn decrypt_aes_key(
        &self,
        wrapped_key: &SplKey,
        key_generation: u32,
        option: u32,
    ) -> Result<SplKey, DispatchError> {
        cmif::decrypt_aes_key(&self.0, wrapped_key, key_generation, option)
    }

    /// Encrypts/decrypts data using AES-CTR mode (cmd 15).
    #[inline]
    pub fn crypt_aes_ctr(
        &self,
        input: &[u8],
        output: &mut [u8],
        keyslot: u32,
        ctr: &SplKey,
    ) -> Result<(), DispatchError> {
        cmif::crypt_aes_ctr(&self.0, input, output, keyslot, ctr)
    }

    /// Computes AES-CMAC over input data (cmd 16).
    #[inline]
    pub fn compute_cmac(&self, input: &[u8], keyslot: u32) -> Result<SplKey, DispatchError> {
        cmif::compute_cmac(&self.0, input, keyslot)
    }

    /// Locks an AES engine keyslot (cmd 21, 2.0.0+).
    #[inline]
    pub fn lock_aes_engine(&self) -> Result<u32, DispatchError> {
        cmif::lock_aes_engine(&self.0)
    }

    /// Unlocks an AES engine keyslot (cmd 22, 2.0.0+).
    #[inline]
    pub fn unlock_aes_engine(&self, keyslot: u32) -> Result<(), DispatchError> {
        cmif::unlock_aes_engine(&self.0, keyslot)
    }

    /// Gets the security engine event handle (cmd 23, 2.0.0+).
    #[inline]
    pub fn get_security_engine_event(&self) -> Result<u32, GetSecurityEngineEventError> {
        cmif::get_security_engine_event(&self.0)
    }

    // --- IManuInterface commands ---

    /// Re-encrypts an RSA key for import (cmd 30, 5.0.0+).
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn encrypt_rsa_key_for_import(
        &self,
        sealed_kek_pre: &SplKey,
        wrapped_key_pre: &SplKey,
        sealed_kek_post: &SplKey,
        wrapped_kek_post: &SplKey,
        option: u32,
        wrapped_rsa_key: &[u8],
        out_wrapped_rsa_key: &mut [u8],
    ) -> Result<(), DispatchError> {
        cmif::manu_encrypt_rsa_key_for_import(
            &self.0,
            sealed_kek_pre,
            wrapped_key_pre,
            sealed_kek_post,
            wrapped_kek_post,
            option,
            wrapped_rsa_key,
            out_wrapped_rsa_key,
        )
    }
}

/// Connects to the manufacturing SPL service (`spl:manu`, 4.0.0+) using CMIF.
pub fn connect_manu_cmif(sm: &SmService) -> Result<SplManuService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(MANU_SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    Ok(SplManuService(Session::new(handle, 0)))
}

// ---------------------------------------------------------------------------
// Connection error
// ---------------------------------------------------------------------------

/// Error returned by all `connect_*_cmif` functions.
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    #[error("failed to get SPL service")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
}
