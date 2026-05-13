//! Security Processor Liaison wire-layout types.

use static_assertions::const_assert_eq;

/// RSA buffer size constant (0x100 bytes).
pub const RSA_BUFFER_SIZE: usize = 0x100;

/// SHA-256 hash size constant (0x20 bytes).
pub const SHA256_HASH_SIZE: usize = 0x20;

/// 128-bit key used in SPL key operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct SplKey {
    pub key: [u8; 0x10],
}

const_assert_eq!(size_of::<SplKey>(), 0x10);

/// Configuration item identifiers for `GetConfig` / `SetConfig`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SplConfigItem {
    DisableProgramVerification = 1,
    DramId = 2,
    SecurityEngineIrqNumber = 3,
    Version = 4,
    HardwareType = 5,
    IsRetail = 6,
    IsRecoveryBoot = 7,
    DeviceId = 8,
    BootReason = 9,
    MemoryArrange = 10,
    IsDebugMode = 11,
    KernelMemoryConfiguration = 12,
    IsChargerHiZModeEnabled = 13,
    IsKiosk = 14,
    NewHardwareType = 15,
    NewKeyGeneration = 16,
    Package2Hash = 17,
}

/// RSA key version for `DecryptRsaPrivateKey` and legacy import commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RsaKeyVersion {
    Deprecated = 0,
    Extended = 1,
}

// --- Wire input structs for IPC commands ---

/// Input for GetConfig (cmd 0).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetConfigIn {
    pub config_item: u32,
}

const_assert_eq!(size_of::<GetConfigIn>(), 0x4);

/// Input for SetConfig (cmd 5).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct SetConfigIn {
    pub config_item: u32,
    pub _pad: u32,
    pub value: u64,
}

const_assert_eq!(size_of::<SetConfigIn>(), 0x10);

/// Input for GenerateAesKek (cmd 2) and DecryptAesKey (cmd 14).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct KeyGenOptionIn {
    pub key: SplKey,
    pub key_generation: u32,
    pub option: u32,
}

const_assert_eq!(size_of::<KeyGenOptionIn>(), 0x18);

/// Input for LoadAesKey (cmd 3).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct LoadAesKeyIn {
    pub sealed_kek: SplKey,
    pub wrapped_key: SplKey,
    pub keyslot: u32,
}

const_assert_eq!(size_of::<LoadAesKeyIn>(), 0x24);

/// Input for GenerateAesKey (cmd 4).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct TwoKeyIn {
    pub sealed_kek: SplKey,
    pub wrapped_key: SplKey,
}

const_assert_eq!(size_of::<TwoKeyIn>(), 0x20);

/// Input for CryptAesCtr (cmd 15).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct CryptAesCtrIn {
    pub ctr: SplKey,
    pub keyslot: u32,
}

const_assert_eq!(size_of::<CryptAesCtrIn>(), 0x14);

/// Input for LoadTitlekey / LoadElicenseKey / LoadContentKey.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct LoadContentKeyIn {
    pub sealed_key: SplKey,
    pub keyslot: u32,
}

const_assert_eq!(size_of::<LoadContentKeyIn>(), 0x14);

/// Input for UnwrapAesWrappedTitlekey (cmd 20).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct UnwrapAesTitlekeyIn {
    pub aes_wrapped_titlekey: SplKey,
    pub key_generation: u32,
}

const_assert_eq!(size_of::<UnwrapAesTitlekeyIn>(), 0x14);

/// Input for DecryptRsaPrivateKey legacy (pre-5.0.0, cmd 13).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct DecryptRsaPrivateKeyLegacyIn {
    pub sealed_kek: SplKey,
    pub wrapped_key: SplKey,
    pub version: u32,
}

const_assert_eq!(size_of::<DecryptRsaPrivateKeyLegacyIn>(), 0x24);

/// Input for ImportSecureExpModKey legacy (pre-5.0.0).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ImportSecureExpModKeyLegacyIn {
    pub sealed_kek: SplKey,
    pub wrapped_key: SplKey,
    pub version: u32,
}

const_assert_eq!(size_of::<ImportSecureExpModKeyLegacyIn>(), 0x24);

/// Input for GenerateSpecificAesKey (cmd 12).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GenerateSpecificAesKeyIn {
    pub wrapped_key: SplKey,
    pub key_generation: u32,
    pub option: u32,
}

const_assert_eq!(size_of::<GenerateSpecificAesKeyIn>(), 0x18);

/// Input for EncryptRsaKeyForImport (cmd 30).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct EncryptRsaKeyForImportIn {
    pub sealed_kek_pre: SplKey,
    pub wrapped_key_pre: SplKey,
    pub sealed_kek_post: SplKey,
    pub wrapped_kek_post: SplKey,
    pub option: u32,
}

const_assert_eq!(size_of::<EncryptRsaKeyForImportIn>(), 0x44);
