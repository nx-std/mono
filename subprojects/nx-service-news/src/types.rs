//! Wire-layout types for the news service.

use static_assertions::const_assert_eq;

/// News topic name (0x20 bytes, null-padded).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct NewsTopicName {
    pub name: [u8; 0x20],
}

const_assert_eq!(core::mem::size_of::<NewsTopicName>(), 0x20);

/// News record (pre-6.0.0 wire format).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct NewsRecordV1 {
    pub news_id: [u8; 0x18],
    pub user_id: [u8; 0x18],
    pub received_at: i64,
    pub read: i32,
    pub newly: i32,
    pub displayed: i32,
    /// Trailing padding to the record's 8-byte alignment. Zero on the wire.
    pub _pad: [u8; 4],
}

const_assert_eq!(core::mem::size_of::<NewsRecordV1>(), 0x48);

/// News record (6.0.0+ wire format).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct NewsRecord {
    pub news_id: [u8; 0x18],
    pub user_id: [u8; 0x18],
    pub topic_id: NewsTopicName,
    pub received_at: i64,
    pub pad_0: i64,
    pub decoration_type: i32,
    pub read: i32,
    pub newly: i32,
    pub displayed: i32,
    pub feedback: i32,
    pub pad_1: i32,
    pub extra_1: i32,
    pub extra_2: i32,
}

const_assert_eq!(core::mem::size_of::<NewsRecord>(), 0x80);

/// Output for `GetSavedataUsage`.
#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub(crate) struct SavedataUsageOut {
    pub current: u64,
    pub total: u64,
}
