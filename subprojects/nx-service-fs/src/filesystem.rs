//! What a filesystem is opened as, and what it reports about itself.
//!
//! This is the vocabulary the commands on an open filesystem speak, not the
//! commands themselves; those are encoded against [`crate::FsFileSystem`].

use static_assertions::const_assert_eq;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FileSystemType {
    Logo = 2,
    ContentControl = 3,
    ContentManual = 4,
    ContentMeta = 5,
    ContentData = 6,
    ApplicationPackage = 7,
    RegisteredUpdate = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FileSystemQueryId {
    SetConcatenationFileAttribute = 0,
    IsValidSignedSystemPartitionOnSdCard = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ContentAttributes {
    None = 0x0,
    All = 0xF,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MountHostOption: u32 {
        const NONE = 0;
        const PSEUDO_CASE_SENSITIVE = 1 << 0;
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FileSystemAttribute {
    pub directory_name_length_max_has_value: u8,
    pub file_name_length_max_has_value: u8,
    pub directory_path_length_max_has_value: u8,
    pub file_path_length_max_has_value: u8,
    pub utf16_create_directory_path_length_max_has_value: u8,
    pub utf16_delete_directory_path_length_max_has_value: u8,
    pub utf16_rename_source_directory_path_length_max_has_value: u8,
    pub utf16_rename_destination_directory_path_length_max_has_value: u8,
    pub utf16_open_directory_path_length_max_has_value: u8,
    pub utf16_directory_name_length_max_has_value: u8,
    pub utf16_file_name_length_max_has_value: u8,
    pub utf16_directory_path_length_max_has_value: u8,
    pub utf16_file_path_length_max_has_value: u8,
    pub reserved1: [u8; 0x1B],
    pub directory_name_length_max: i32,
    pub file_name_length_max: i32,
    pub directory_path_length_max: i32,
    pub file_path_length_max: i32,
    pub utf16_create_directory_path_length_max: i32,
    pub utf16_delete_directory_path_length_max: i32,
    pub utf16_rename_source_directory_path_length_max: i32,
    pub utf16_rename_destination_directory_path_length_max: i32,
    pub utf16_open_directory_path_length_max: i32,
    pub utf16_directory_name_length_max: i32,
    pub utf16_file_name_length_max: i32,
    pub utf16_directory_path_length_max: i32,
    pub utf16_file_path_length_max: i32,
    pub reserved2: [u8; 0x64],
}
const_assert_eq!(core::mem::size_of::<FileSystemAttribute>(), 0xC0);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenFileSystemWithPatchIn {
    pub fs_type: u32,
    pub id: u64,
}
const_assert_eq!(core::mem::size_of::<OpenFileSystemWithPatchIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenFileSystemWithIdIn {
    pub fs_type: u32,
    pub id: u64,
}
const_assert_eq!(core::mem::size_of::<OpenFileSystemWithIdIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenFileSystemWithIdV16In {
    pub attr: u8,
    pub _pad: [u8; 3],
    pub fs_type: u32,
    pub id: u64,
}
const_assert_eq!(core::mem::size_of::<OpenFileSystemWithIdV16In>(), 0x10);
