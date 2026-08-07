//! Wire-layout types for the filesystem loader service.

use static_assertions::const_assert_eq;

/// Maximum path length for filesystem operations (0x301 = 769 bytes).
pub const FS_MAX_PATH: usize = 0x301;

/// Code information returned by `OpenCodeFileSystem` (10.0.0+).
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct FsCodeInfo {
    /// RSA-2048 signature.
    pub signature: [u8; 0x100],
    /// SHA-256 hash.
    pub hash: [u8; 0x20],
    /// Whether the code is signed.
    pub is_signed: u8,
    /// Reserved padding.
    pub reserved: [u8; 3],
}

const_assert_eq!(size_of::<FsCodeInfo>(), 0x124);

/// Input payload for `OpenCodeFileSystem` (pre-16.0.0 variants).
///
/// Wire layout: just a title ID (`u64`).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct OpenCodeFileSystemTidIn {
    pub tid: u64,
}

const_assert_eq!(size_of::<OpenCodeFileSystemTidIn>(), 0x8);

/// Input payload for `OpenCodeFileSystem` (16.0.0–19.x).
///
/// Wire layout: content attributes (`u8`) + padding + title ID (`u64`).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct OpenCodeFileSystemAttrIn {
    pub content_attributes: u8,
    _pad: [u8; 7],
    pub tid: u64,
}

const_assert_eq!(size_of::<OpenCodeFileSystemAttrIn>(), 0x10);

impl OpenCodeFileSystemAttrIn {
    pub const fn new(content_attributes: u8, tid: u64) -> Self {
        Self {
            content_attributes,
            _pad: [0; 7],
            tid,
        }
    }
}

/// Input payload for `OpenCodeFileSystem` (20.0.0+).
///
/// Wire layout: content attributes (`u8`) + storage ID (`u8`) + padding +
/// title ID (`u64`).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct OpenCodeFileSystemV20In {
    pub content_attributes: u8,
    pub storage_id: u8,
    _pad: [u8; 6],
    pub tid: u64,
}

const_assert_eq!(size_of::<OpenCodeFileSystemV20In>(), 0x10);

impl OpenCodeFileSystemV20In {
    pub const fn new(content_attributes: u8, storage_id: u8, tid: u64) -> Self {
        Self {
            content_attributes,
            storage_id,
            _pad: [0; 6],
            tid,
        }
    }
}

/// Input payload for `SetCurrentProcess`.
///
/// Wire layout: 64-bit PID placeholder (kernel fills in the real PID).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct SetCurrentProcessIn {
    pub pid_placeholder: u64,
}

const_assert_eq!(size_of::<SetCurrentProcessIn>(), 0x8);
