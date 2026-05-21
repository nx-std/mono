//! CMIF protocol operations for the Security Processor Liaison service.

use core::{mem::size_of, ptr};

use nx_sf::service::{BufferAttr, DispatchError, OutHandleAttr, Session};

use crate::{
    dispatch::{dispatch_in, dispatch_in_out, dispatch_out},
    proto,
    types::{
        CryptAesCtrIn, DecryptRsaPrivateKeyLegacyIn, EncryptRsaKeyForImportIn,
        GenerateSpecificAesKeyIn, GetConfigIn, ImportSecureExpModKeyLegacyIn, KeyGenOptionIn,
        LoadAesKeyIn, LoadContentKeyIn, RSA_BUFFER_SIZE, RsaKeyVersion, SHA256_HASH_SIZE,
        SetConfigIn, SplKey, TwoKeyIn, UnwrapAesTitlekeyIn,
    },
};

// ---------------------------------------------------------------------------
// IGeneralInterface commands
// ---------------------------------------------------------------------------

/// GetConfig (cmd 0).
pub(crate) fn get_config(service: &Session, config_item: u32) -> Result<u64, DispatchError> {
    let input = GetConfigIn { config_item };
    dispatch_in_out(service, proto::GET_CONFIG, &input)
}

/// UserExpMod (cmd 1).
pub(crate) fn user_exp_mod(
    service: &Session,
    input: &[u8; RSA_BUFFER_SIZE],
    modulus: &[u8; RSA_BUFFER_SIZE],
    exp: &[u8],
    dst: &mut [u8; RSA_BUFFER_SIZE],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::USER_EXP_MOD)
        .out_buffer(dst, BufferAttr::HIPC_POINTER)
        .in_buffer(input, BufferAttr::HIPC_POINTER)
        .in_buffer(exp, BufferAttr::HIPC_POINTER)
        .in_buffer(modulus, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// SetConfig (cmd 5).
pub(crate) fn set_config(
    service: &Session,
    config_item: u32,
    value: u64,
) -> Result<(), DispatchError> {
    let input = SetConfigIn {
        config_item,
        _pad: 0,
        value,
    };
    dispatch_in(service, proto::SET_CONFIG, &input)
}

/// GetRandomBytes (cmd 7).
pub(crate) fn get_random_bytes(service: &Session, out: &mut [u8]) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::GET_RANDOM_BYTES)
        .out_buffer(out, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// IsDevelopment (cmd 11).
pub(crate) fn is_development(service: &Session) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(service, proto::IS_DEVELOPMENT)?;
    Ok(val & 1 != 0)
}

/// SetBootReason (cmd 24, 3.0.0+).
pub(crate) fn set_boot_reason(service: &Session, value: u32) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_BOOT_REASON, &value)
}

/// GetBootReason (cmd 25, 3.0.0+).
pub(crate) fn get_boot_reason(service: &Session) -> Result<u32, DispatchError> {
    dispatch_out(service, proto::GET_BOOT_REASON)
}

// ---------------------------------------------------------------------------
// ICryptoInterface commands
// ---------------------------------------------------------------------------

/// GenerateAesKek (cmd 2).
pub(crate) fn generate_aes_kek(
    service: &Session,
    wrapped_kek: &SplKey,
    key_generation: u32,
    option: u32,
) -> Result<SplKey, DispatchError> {
    let input = KeyGenOptionIn {
        key: *wrapped_kek,
        key_generation,
        option,
    };
    dispatch_in_out(service, proto::GENERATE_AES_KEK, &input)
}

/// LoadAesKey (cmd 3).
pub(crate) fn load_aes_key(
    service: &Session,
    sealed_kek: &SplKey,
    wrapped_key: &SplKey,
    keyslot: u32,
) -> Result<(), DispatchError> {
    let input = LoadAesKeyIn {
        sealed_kek: *sealed_kek,
        wrapped_key: *wrapped_key,
        keyslot,
    };
    dispatch_in(service, proto::LOAD_AES_KEY, &input)
}

/// GenerateAesKey (cmd 4).
pub(crate) fn generate_aes_key(
    service: &Session,
    sealed_kek: &SplKey,
    wrapped_key: &SplKey,
) -> Result<SplKey, DispatchError> {
    let input = TwoKeyIn {
        sealed_kek: *sealed_kek,
        wrapped_key: *wrapped_key,
    };
    dispatch_in_out(service, proto::GENERATE_AES_KEY, &input)
}

/// DecryptAesKey (cmd 14).
pub(crate) fn decrypt_aes_key(
    service: &Session,
    wrapped_key: &SplKey,
    key_generation: u32,
    option: u32,
) -> Result<SplKey, DispatchError> {
    let input = KeyGenOptionIn {
        key: *wrapped_key,
        key_generation,
        option,
    };
    dispatch_in_out(service, proto::DECRYPT_AES_KEY, &input)
}

/// CryptAesCtr (cmd 15).
pub(crate) fn crypt_aes_ctr(
    service: &Session,
    input_data: &[u8],
    output: &mut [u8],
    keyslot: u32,
    ctr: &SplKey,
) -> Result<(), DispatchError> {
    let input = CryptAesCtrIn { ctr: *ctr, keyslot };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<CryptAesCtrIn>()` bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<CryptAesCtrIn>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::CRYPT_AES_CTR)
        .in_raw(in_bytes)
        .out_buffer(
            output,
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE),
        )
        .in_buffer(
            input_data,
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// ComputeCmac (cmd 16).
pub(crate) fn compute_cmac(
    service: &Session,
    input_data: &[u8],
    keyslot: u32,
) -> Result<SplKey, DispatchError> {
    // SAFETY: `keyslot` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<u32>()` bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const keyslot).cast::<u8>(), size_of::<u32>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::COMPUTE_CMAC)
        .in_raw(in_bytes)
        .out_size(size_of::<SplKey>())
        .in_buffer(input_data, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<SplKey>() bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<SplKey>()) })
}

/// LockAesEngine (cmd 21, 2.0.0+).
pub(crate) fn lock_aes_engine(service: &Session) -> Result<u32, DispatchError> {
    dispatch_out(service, proto::LOCK_AES_ENGINE)
}

/// UnlockAesEngine (cmd 22, 2.0.0+).
pub(crate) fn unlock_aes_engine(service: &Session, keyslot: u32) -> Result<(), DispatchError> {
    dispatch_in(service, proto::UNLOCK_AES_ENGINE, &keyslot)
}

/// GetSecurityEngineEvent (cmd 23, 2.0.0+).
pub(crate) fn get_security_engine_event(
    service: &Session,
) -> Result<u32, GetSecurityEngineEventError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::GET_SECURITY_ENGINE_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(GetSecurityEngineEventError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(GetSecurityEngineEventError::MissingHandle);
    }

    Ok(result.copy_handles[0])
}

// ---------------------------------------------------------------------------
// IRsaService commands (available on crypto/ssl/es/fs services)
// ---------------------------------------------------------------------------

/// DecryptRsaPrivateKey legacy (pre-5.0.0, cmd 13).
pub(crate) fn decrypt_rsa_private_key_legacy(
    service: &Session,
    sealed_kek: &SplKey,
    wrapped_key: &SplKey,
    version: RsaKeyVersion,
    wrapped_rsa_key: &[u8],
    dst: &mut [u8],
) -> Result<(), DispatchError> {
    let input = DecryptRsaPrivateKeyLegacyIn {
        sealed_kek: *sealed_kek,
        wrapped_key: *wrapped_key,
        version: version as u32,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<DecryptRsaPrivateKeyLegacyIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::DECRYPT_RSA_PRIVATE_KEY)
        .in_raw(in_bytes)
        .out_buffer(dst, BufferAttr::HIPC_POINTER)
        .in_buffer(wrapped_rsa_key, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// DecryptRsaPrivateKey (5.0.0+, cmd 13).
pub(crate) fn decrypt_rsa_private_key(
    service: &Session,
    sealed_kek: &SplKey,
    wrapped_key: &SplKey,
    wrapped_rsa_key: &[u8],
    dst: &mut [u8],
) -> Result<(), DispatchError> {
    let input = TwoKeyIn {
        sealed_kek: *sealed_kek,
        wrapped_key: *wrapped_key,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<TwoKeyIn>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::DECRYPT_RSA_PRIVATE_KEY)
        .in_raw(in_bytes)
        .out_buffer(dst, BufferAttr::HIPC_POINTER)
        .in_buffer(wrapped_rsa_key, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// ISslInterface commands
// ---------------------------------------------------------------------------

/// LoadSecureExpModKey for SSL (cmd 26, 5.0.0+).
pub(crate) fn ssl_load_secure_exp_mod_key(
    service: &Session,
    sealed_kek: &SplKey,
    wrapped_key: &SplKey,
    wrapped_rsa_key: &[u8],
) -> Result<(), DispatchError> {
    import_secure_exp_mod_key(
        service,
        proto::SSL_LOAD_SECURE_EXP_MOD_KEY,
        sealed_kek,
        wrapped_key,
        wrapped_rsa_key,
    )
}

/// SecureExpMod for SSL (cmd 27, 5.0.0+).
pub(crate) fn ssl_secure_exp_mod(
    service: &Session,
    input: &[u8; RSA_BUFFER_SIZE],
    modulus: &[u8; RSA_BUFFER_SIZE],
    dst: &mut [u8; RSA_BUFFER_SIZE],
) -> Result<(), DispatchError> {
    secure_exp_mod(service, proto::SSL_SECURE_EXP_MOD, input, modulus, dst)
}

// ---------------------------------------------------------------------------
// IEsInterface commands
// ---------------------------------------------------------------------------

/// LoadRsaOaepKey legacy (pre-5.0.0, cmd 17).
pub(crate) fn es_load_rsa_oaep_key_legacy(
    service: &Session,
    sealed_kek: &SplKey,
    wrapped_key: &SplKey,
    wrapped_rsa_key: &[u8],
    version: RsaKeyVersion,
) -> Result<(), DispatchError> {
    import_secure_exp_mod_key_legacy(
        service,
        proto::ES_LOAD_RSA_OAEP_KEY,
        sealed_kek,
        wrapped_key,
        wrapped_rsa_key,
        version,
    )
}

/// LoadRsaOaepKey (5.0.0+, cmd 17).
pub(crate) fn es_load_rsa_oaep_key(
    service: &Session,
    sealed_kek: &SplKey,
    wrapped_key: &SplKey,
    wrapped_rsa_key: &[u8],
) -> Result<(), DispatchError> {
    import_secure_exp_mod_key(
        service,
        proto::ES_LOAD_RSA_OAEP_KEY,
        sealed_kek,
        wrapped_key,
        wrapped_rsa_key,
    )
}

/// UnwrapRsaOaepWrappedTitlekey (cmd 18).
pub(crate) fn es_unwrap_rsa_oaep_wrapped_titlekey(
    service: &Session,
    rsa_wrapped_titlekey: &[u8; RSA_BUFFER_SIZE],
    modulus: &[u8; RSA_BUFFER_SIZE],
    label_hash: &[u8],
    key_generation: u32,
) -> Result<SplKey, DispatchError> {
    unwrap_rsa_oaep_wrapped_key(
        service,
        proto::ES_UNWRAP_RSA_OAEP_WRAPPED_TITLEKEY,
        rsa_wrapped_titlekey,
        modulus,
        label_hash,
        key_generation,
    )
}

/// UnwrapAesWrappedTitlekey (cmd 20, 2.0.0+).
pub(crate) fn es_unwrap_aes_wrapped_titlekey(
    service: &Session,
    aes_wrapped_titlekey: &SplKey,
    key_generation: u32,
) -> Result<SplKey, DispatchError> {
    let input = UnwrapAesTitlekeyIn {
        aes_wrapped_titlekey: *aes_wrapped_titlekey,
        key_generation,
    };
    dispatch_in_out(service, proto::ES_UNWRAP_AES_WRAPPED_TITLEKEY, &input)
}

/// LoadSecureExpModKey for ES (cmd 28, 5.0.0+).
pub(crate) fn es_load_secure_exp_mod_key(
    service: &Session,
    sealed_kek: &SplKey,
    wrapped_key: &SplKey,
    wrapped_rsa_key: &[u8],
) -> Result<(), DispatchError> {
    import_secure_exp_mod_key(
        service,
        proto::ES_LOAD_SECURE_EXP_MOD_KEY,
        sealed_kek,
        wrapped_key,
        wrapped_rsa_key,
    )
}

/// SecureExpMod for ES (cmd 29, 5.0.0+).
pub(crate) fn es_secure_exp_mod(
    service: &Session,
    input: &[u8; RSA_BUFFER_SIZE],
    modulus: &[u8; RSA_BUFFER_SIZE],
    dst: &mut [u8; RSA_BUFFER_SIZE],
) -> Result<(), DispatchError> {
    secure_exp_mod(service, proto::ES_SECURE_EXP_MOD, input, modulus, dst)
}

/// UnwrapElicenseKey (cmd 31, 6.0.0+).
pub(crate) fn es_unwrap_elicense_key(
    service: &Session,
    rsa_wrapped_key: &[u8; RSA_BUFFER_SIZE],
    modulus: &[u8; RSA_BUFFER_SIZE],
    label_hash: &[u8],
    key_generation: u32,
) -> Result<SplKey, DispatchError> {
    unwrap_rsa_oaep_wrapped_key(
        service,
        proto::ES_UNWRAP_ELICENSE_KEY,
        rsa_wrapped_key,
        modulus,
        label_hash,
        key_generation,
    )
}

/// LoadElicenseKey (cmd 32, 6.0.0+).
pub(crate) fn es_load_elicense_key(
    service: &Session,
    sealed_key: &SplKey,
    keyslot: u32,
) -> Result<(), DispatchError> {
    let input = LoadContentKeyIn {
        sealed_key: *sealed_key,
        keyslot,
    };
    dispatch_in(service, proto::ES_LOAD_ELICENSE_KEY, &input)
}

// ---------------------------------------------------------------------------
// IFsInterface commands
// ---------------------------------------------------------------------------

/// LoadSecureExpModKey for FS legacy (pre-5.0.0, cmd 9).
pub(crate) fn fs_load_secure_exp_mod_key_legacy(
    service: &Session,
    sealed_kek: &SplKey,
    wrapped_key: &SplKey,
    wrapped_rsa_key: &[u8],
    version: RsaKeyVersion,
) -> Result<(), DispatchError> {
    import_secure_exp_mod_key_legacy(
        service,
        proto::FS_LOAD_SECURE_EXP_MOD_KEY,
        sealed_kek,
        wrapped_key,
        wrapped_rsa_key,
        version,
    )
}

/// LoadSecureExpModKey for FS (5.0.0+, cmd 9).
pub(crate) fn fs_load_secure_exp_mod_key(
    service: &Session,
    sealed_kek: &SplKey,
    wrapped_key: &SplKey,
    wrapped_rsa_key: &[u8],
) -> Result<(), DispatchError> {
    import_secure_exp_mod_key(
        service,
        proto::FS_LOAD_SECURE_EXP_MOD_KEY,
        sealed_kek,
        wrapped_key,
        wrapped_rsa_key,
    )
}

/// SecureExpMod for FS (cmd 10).
pub(crate) fn fs_secure_exp_mod(
    service: &Session,
    input: &[u8; RSA_BUFFER_SIZE],
    modulus: &[u8; RSA_BUFFER_SIZE],
    dst: &mut [u8; RSA_BUFFER_SIZE],
) -> Result<(), DispatchError> {
    secure_exp_mod(service, proto::FS_SECURE_EXP_MOD, input, modulus, dst)
}

/// GenerateSpecificAesKey (cmd 12).
pub(crate) fn fs_generate_specific_aes_key(
    service: &Session,
    wrapped_key: &SplKey,
    key_generation: u32,
    option: u32,
) -> Result<SplKey, DispatchError> {
    let input = GenerateSpecificAesKeyIn {
        wrapped_key: *wrapped_key,
        key_generation,
        option,
    };
    dispatch_in_out(service, proto::GENERATE_SPECIFIC_AES_KEY, &input)
}

/// LoadTitlekey (cmd 19).
pub(crate) fn fs_load_titlekey(
    service: &Session,
    sealed_titlekey: &SplKey,
    keyslot: u32,
) -> Result<(), DispatchError> {
    let input = LoadContentKeyIn {
        sealed_key: *sealed_titlekey,
        keyslot,
    };
    dispatch_in(service, proto::LOAD_TITLEKEY, &input)
}

/// GetPackage2Hash (cmd 31, 5.0.0+).
pub(crate) fn fs_get_package2_hash(
    service: &Session,
    out_hash: &mut [u8; SHA256_HASH_SIZE],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::FS_GET_PACKAGE2_HASH)
        .out_buffer(out_hash, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// IManuInterface commands
// ---------------------------------------------------------------------------

/// EncryptRsaKeyForImport (cmd 30, 5.0.0+).
#[allow(clippy::too_many_arguments)]
pub(crate) fn manu_encrypt_rsa_key_for_import(
    service: &Session,
    sealed_kek_pre: &SplKey,
    wrapped_key_pre: &SplKey,
    sealed_kek_post: &SplKey,
    wrapped_kek_post: &SplKey,
    option: u32,
    wrapped_rsa_key: &[u8],
    out_wrapped_rsa_key: &mut [u8],
) -> Result<(), DispatchError> {
    let input = EncryptRsaKeyForImportIn {
        sealed_kek_pre: *sealed_kek_pre,
        wrapped_key_pre: *wrapped_key_pre,
        sealed_kek_post: *sealed_kek_post,
        wrapped_kek_post: *wrapped_kek_post,
        option,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<EncryptRsaKeyForImportIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::MANU_ENCRYPT_RSA_KEY_FOR_IMPORT)
        .in_raw(in_bytes)
        .out_buffer(out_wrapped_rsa_key, BufferAttr::HIPC_POINTER)
        .in_buffer(wrapped_rsa_key, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// Shared dispatch helpers
// ---------------------------------------------------------------------------

/// SecureExpMod dispatch (used by SSL cmd 27, ES cmd 29, FS cmd 10).
fn secure_exp_mod(
    service: &Session,
    cmd_id: u32,
    input: &[u8; RSA_BUFFER_SIZE],
    modulus: &[u8; RSA_BUFFER_SIZE],
    dst: &mut [u8; RSA_BUFFER_SIZE],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(cmd_id)
        .out_buffer(dst, BufferAttr::HIPC_POINTER)
        .in_buffer(input, BufferAttr::HIPC_POINTER)
        .in_buffer(modulus, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// ImportSecureExpModKey (5.0.0+, used by SSL cmd 26, ES cmds 17/28, FS cmd 9).
fn import_secure_exp_mod_key(
    service: &Session,
    cmd_id: u32,
    sealed_kek: &SplKey,
    wrapped_key: &SplKey,
    wrapped_rsa_key: &[u8],
) -> Result<(), DispatchError> {
    let input = TwoKeyIn {
        sealed_kek: *sealed_kek,
        wrapped_key: *wrapped_key,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<TwoKeyIn>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .in_buffer(wrapped_rsa_key, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// ImportSecureExpModKey legacy (pre-5.0.0, used by ES cmd 17, FS cmd 9).
fn import_secure_exp_mod_key_legacy(
    service: &Session,
    cmd_id: u32,
    sealed_kek: &SplKey,
    wrapped_key: &SplKey,
    wrapped_rsa_key: &[u8],
    version: RsaKeyVersion,
) -> Result<(), DispatchError> {
    let input = ImportSecureExpModKeyLegacyIn {
        sealed_kek: *sealed_kek,
        wrapped_key: *wrapped_key,
        version: version as u32,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<ImportSecureExpModKeyLegacyIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .in_buffer(wrapped_rsa_key, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// UnwrapRsaOaepWrappedKey dispatch (used by ES cmds 18, 31).
fn unwrap_rsa_oaep_wrapped_key(
    service: &Session,
    cmd_id: u32,
    rsa_wrapped_key: &[u8; RSA_BUFFER_SIZE],
    modulus: &[u8; RSA_BUFFER_SIZE],
    label_hash: &[u8],
    key_generation: u32,
) -> Result<SplKey, DispatchError> {
    // SAFETY: `key_generation` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const key_generation).cast::<u8>(), size_of::<u32>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<SplKey>())
        .in_buffer(rsa_wrapped_key, BufferAttr::HIPC_POINTER)
        .in_buffer(modulus, BufferAttr::HIPC_POINTER)
        .in_buffer(label_hash, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<SplKey>() bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<SplKey>()) })
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned by [`get_security_engine_event`].
#[derive(Debug, thiserror::Error)]
pub enum GetSecurityEngineEventError {
    #[error("failed to dispatch GetSecurityEngineEvent")]
    Dispatch(#[source] DispatchError),
    #[error("response did not include expected copy handle")]
    MissingHandle,
}
