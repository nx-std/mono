//! Overlay notification wire-layout types.

use static_assertions::const_assert_eq;

/// Source name for overlay notifications.
///
/// Official software always uses the name "overlay".
#[derive(Debug, Clone, Copy, PartialEq, Eq, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct OvlnSourceName {
    pub name: [u8; 0x10],
}

const_assert_eq!(size_of::<OvlnSourceName>(), 0x10);

impl OvlnSourceName {
    /// Creates a source name from a byte slice, zero-padding if shorter than 16 bytes.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut name = [0u8; 0x10];
        let len = bytes.len().min(0x10);
        name[..len].copy_from_slice(&bytes[..len]);
        Self { name }
    }
}

/// Raw overlay notification message.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::IntoBytes,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct OvlnRawMessage {
    pub tag: u32,
    pub data_size: u32,
    pub data: [u8; 0x78],
}

const_assert_eq!(size_of::<OvlnRawMessage>(), 0x80);

/// Queue attribute for sender creation.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct OvlnQueueAttribute {
    pub queue_length: u32,
    pub reserved: [u8; 4],
}

const_assert_eq!(size_of::<OvlnQueueAttribute>(), 0x08);

/// Send option controlling enqueue position and overflow behavior.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct OvlnSendOption {
    pub enqueue_position: u8,
    pub overflow_option: u8,
    pub reserved: [u8; 6],
}

const_assert_eq!(size_of::<OvlnSendOption>(), 0x08);

/// Enqueue position for sent messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OvlnEnqueuePosition {
    Front = 0,
    Back = 1,
}

/// Overflow option when the send queue is full.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OvlnOverflowOption {
    Error = 0,
    RemoveFront = 1,
    RemoveBack = 2,
    Block = 3,
}

/// Result of [`OvlnReceiver::receive_with_tick`](crate::OvlnReceiver::receive_with_tick).
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct ReceiveWithTickOut {
    pub message: OvlnRawMessage,
    pub tick: i64,
}

const_assert_eq!(size_of::<ReceiveWithTickOut>(), 0x88);
