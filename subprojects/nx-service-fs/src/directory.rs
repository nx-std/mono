//! How a directory is opened and what one entry of it looks like on the wire.

use static_assertions::const_assert_eq;

use crate::path::FS_MAX_PATH;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum DirEntryType {
    Dir = 0,
    File = 1,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DirOpenMode: u32 {
        const READ_DIRS    = 1 << 0;
        const READ_FILES   = 1 << 1;
        const NO_FILE_SIZE = 1 << 31;
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DirectoryEntry {
    pub name: [u8; FS_MAX_PATH],
    pub pad: [u8; 3],
    pub entry_type: i8,
    pub pad2: [u8; 3],
    pub file_size: i64,
}
const_assert_eq!(core::mem::size_of::<DirectoryEntry>(), 0x310);
