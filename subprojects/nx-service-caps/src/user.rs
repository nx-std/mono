//! The users a saved screenshot is attributed to.

use static_assertions::const_assert_eq;

/// Maximum number of user IDs in a [`UserIdList`].
pub const USER_LIST_SIZE: usize = 8;

/// Account user ID (128-bit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct AccountUid {
    /// The 128-bit ID, low word first.
    pub uid: [u64; 2],
}

const_assert_eq!(size_of::<AccountUid>(), 0x10);

/// List of user IDs attached to a screenshot.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct UserIdList {
    /// The IDs; only the first `count` entries are meaningful.
    pub uids: [AccountUid; USER_LIST_SIZE],
    /// Number of entries of `uids` that are set.
    pub count: u8,
    /// Padding the wire form carries after the count.
    pub pad: [u8; 7],
}

const_assert_eq!(size_of::<UserIdList>(), 0x88);
