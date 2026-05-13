//! Wire-layout types for the location resolver service.

use static_assertions::const_assert_eq;

/// Maximum path length for location resolver operations (same as FS_MAX_PATH).
pub const LR_MAX_PATH: usize = 0x301;

/// NCM storage identifier.
///
/// Determines which storage device to resolve content locations from.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageId {
    None = 0,
    Host = 1,
    GameCard = 2,
    BuiltInSystem = 3,
    BuiltInUser = 4,
    SdCard = 5,
    Any = 6,
}

/// Input payload for redirect-application commands (9.0.0+ wire format).
///
/// Wire layout: two title IDs.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct RedirectApplicationIn {
    pub tid: u64,
    pub tid2: u64,
}

const_assert_eq!(size_of::<RedirectApplicationIn>(), 0x10);
