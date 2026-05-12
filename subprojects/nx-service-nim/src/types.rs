//! Wire-layout types for the NIM service.

/// Unique identifier for a system update task.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemUpdateTaskId {
    pub uuid: [u8; 0x10],
}

static_assertions::const_assert_eq!(size_of::<SystemUpdateTaskId>(), 0x10);
