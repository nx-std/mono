//! How a file is opened and written, and the timestamps it carries.
//!
//! Reading and writing a byte range is one operation on a file and another on a
//! storage, so the range vocabulary the two share lives in [`crate::range`].

use static_assertions::const_assert_eq;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenMode: u32 {
        const READ   = 1 << 0;
        const WRITE  = 1 << 1;
        const APPEND = 1 << 2;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CreateOption: u32 {
        const BIG_FILE = 1 << 0;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ReadOption: u32 {
        const NONE = 0;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct WriteOption: u32 {
        const NONE  = 0;
        const FLUSH = 1 << 0;
    }
}

#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct TimeStampRaw {
    pub created: u64,
    pub modified: u64,
    pub accessed: u64,
    pub is_valid: u8,
    pub padding: [u8; 7],
}
const_assert_eq!(core::mem::size_of::<TimeStampRaw>(), 0x20);

#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct CreateFileIn {
    pub option: u32,
    pub _pad: u32,
    pub size: i64,
}
const_assert_eq!(core::mem::size_of::<CreateFileIn>(), 0x10);

#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct FileReadIn {
    pub option: u32,
    pub pad: u32,
    pub offset: i64,
    pub read_size: u64,
}
const_assert_eq!(core::mem::size_of::<FileReadIn>(), 0x18);

#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct FileWriteIn {
    pub option: u32,
    pub pad: u32,
    pub offset: i64,
    pub write_size: u64,
}
const_assert_eq!(core::mem::size_of::<FileWriteIn>(), 0x18);
