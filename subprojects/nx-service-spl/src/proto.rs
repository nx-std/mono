//! Security Processor Liaison service protocol constants.

use nx_sf::ServiceName;

/// Service name for `spl:` (IGeneralInterface).
pub const GENERAL_SERVICE_NAME: ServiceName = ServiceName::new_truncate("spl:");

/// Service name for `spl:mig` (ICryptoInterface, 4.0.0+).
pub const CRYPTO_SERVICE_NAME: ServiceName = ServiceName::new_truncate("spl:mig");

/// Service name for `spl:ssl` (ISslInterface, 4.0.0+).
pub const SSL_SERVICE_NAME: ServiceName = ServiceName::new_truncate("spl:ssl");

/// Service name for `spl:es` (IEsInterface, 4.0.0+).
pub const ES_SERVICE_NAME: ServiceName = ServiceName::new_truncate("spl:es");

/// Service name for `spl:fs` (IFsInterface, 4.0.0+).
pub const FS_SERVICE_NAME: ServiceName = ServiceName::new_truncate("spl:fs");

/// Service name for `spl:manu` (IManuInterface, 4.0.0+).
pub const MANU_SERVICE_NAME: ServiceName = ServiceName::new_truncate("spl:manu");

// IGeneralInterface commands

/// GetConfig (cmd 0).
pub const GET_CONFIG: u32 = 0;

/// UserExpMod (cmd 1).
pub const USER_EXP_MOD: u32 = 1;

/// SetConfig (cmd 5).
pub const SET_CONFIG: u32 = 5;

/// GetRandomBytes (cmd 7).
pub const GET_RANDOM_BYTES: u32 = 7;

/// IsDevelopment (cmd 11).
pub const IS_DEVELOPMENT: u32 = 11;

/// SetBootReason (cmd 24, 3.0.0+).
pub const SET_BOOT_REASON: u32 = 24;

/// GetBootReason (cmd 25, 3.0.0+).
pub const GET_BOOT_REASON: u32 = 25;

// ICryptoInterface commands

/// GenerateAesKek (cmd 2).
pub const GENERATE_AES_KEK: u32 = 2;

/// LoadAesKey (cmd 3).
pub const LOAD_AES_KEY: u32 = 3;

/// GenerateAesKey (cmd 4).
pub const GENERATE_AES_KEY: u32 = 4;

/// DecryptRsaPrivateKey (cmd 13).
pub const DECRYPT_RSA_PRIVATE_KEY: u32 = 13;

/// DecryptAesKey (cmd 14).
pub const DECRYPT_AES_KEY: u32 = 14;

/// CryptAesCtr (cmd 15).
pub const CRYPT_AES_CTR: u32 = 15;

/// ComputeCmac (cmd 16).
pub const COMPUTE_CMAC: u32 = 16;

/// LockAesEngine (cmd 21, 2.0.0+).
pub const LOCK_AES_ENGINE: u32 = 21;

/// UnlockAesEngine (cmd 22, 2.0.0+).
pub const UNLOCK_AES_ENGINE: u32 = 22;

/// GetSecurityEngineEvent (cmd 23, 2.0.0+).
pub const GET_SECURITY_ENGINE_EVENT: u32 = 23;

// IFsInterface commands

/// ImportLotusKey / LoadSecureExpModKey for FS (cmd 9).
pub const FS_LOAD_SECURE_EXP_MOD_KEY: u32 = 9;

/// SecureExpMod for FS (cmd 10).
pub const FS_SECURE_EXP_MOD: u32 = 10;

/// GenerateSpecificAesKey (cmd 12).
pub const GENERATE_SPECIFIC_AES_KEY: u32 = 12;

/// LoadTitlekey (cmd 19).
pub const LOAD_TITLEKEY: u32 = 19;

// IEsInterface commands

/// LoadEsDeviceKey / LoadRsaOaepKey (cmd 17).
pub const ES_LOAD_RSA_OAEP_KEY: u32 = 17;

/// UnwrapTitlekey / UnwrapRsaOaepWrappedTitlekey (cmd 18).
pub const ES_UNWRAP_RSA_OAEP_WRAPPED_TITLEKEY: u32 = 18;

/// UnwrapAesWrappedTitlekey (cmd 20, 2.0.0+).
pub const ES_UNWRAP_AES_WRAPPED_TITLEKEY: u32 = 20;

// ISslInterface commands

/// LoadSslKey / LoadSecureExpModKey for SSL (cmd 26, 5.0.0+).
pub const SSL_LOAD_SECURE_EXP_MOD_KEY: u32 = 26;

/// SecureExpMod for SSL (cmd 27, 5.0.0+).
pub const SSL_SECURE_EXP_MOD: u32 = 27;

// IEsInterface additional commands

/// LoadSecureExpModKey for ES (cmd 28, 5.0.0+).
pub const ES_LOAD_SECURE_EXP_MOD_KEY: u32 = 28;

/// SecureExpMod for ES (cmd 29, 5.0.0+).
pub const ES_SECURE_EXP_MOD: u32 = 29;

// IManuInterface commands

/// ReEncryptRsaPrivateKey / EncryptRsaKeyForImport (cmd 30, 5.0.0+).
pub const MANU_ENCRYPT_RSA_KEY_FOR_IMPORT: u32 = 30;

// IEsInterface additional commands (6.0.0+)

/// UnwrapElicenseKey (cmd 31, 6.0.0+).
pub const ES_UNWRAP_ELICENSE_KEY: u32 = 31;

// IFsInterface additional commands (5.0.0+)

/// GetPackage2Hash (cmd 31, 5.0.0+ FS only).
pub const FS_GET_PACKAGE2_HASH: u32 = 31;

/// LoadElicenseKey (cmd 32, 6.0.0+).
pub const ES_LOAD_ELICENSE_KEY: u32 = 32;
