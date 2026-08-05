//! The areas a filesystem or storage is opened from, and how one is addressed.
//!
//! Each id names a different partitioning of the device's storage, so which one
//! a command takes is what says where it is looking.

use static_assertions::const_assert_eq;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BisPartitionId {
    BootPartition1Root = 0,
    BootPartition2Root = 10,
    UserDataRoot = 20,
    BootConfigAndPackage2Part1 = 21,
    BootConfigAndPackage2Part2 = 22,
    BootConfigAndPackage2Part3 = 23,
    BootConfigAndPackage2Part4 = 24,
    BootConfigAndPackage2Part5 = 25,
    BootConfigAndPackage2Part6 = 26,
    CalibrationBinary = 27,
    CalibrationFile = 28,
    SafeMode = 29,
    User = 30,
    System = 31,
    SystemProperEncryption = 32,
    SystemProperPartition = 33,
    SignedSystemPartitionOnSafeMode = 34,
    DeviceTreeBlob = 35,
    System0 = 36,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ContentStorageId {
    System = 0,
    User = 1,
    SdCard = 2,
    System0 = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CustomStorageId {
    System = 0,
    SdCard = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ImageDirectoryId {
    Nand = 0,
    Sd = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NcmStorageId {
    None = 0,
    Host = 1,
    GameCard = 2,
    BuiltinSystem = 3,
    BuiltinUser = 4,
    SdCard = 5,
    Any = 6,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenDataStorageByDataIdIn {
    pub storage_id: u8,
    pub _pad: [u8; 7],
    pub data_id: u64,
}
const_assert_eq!(core::mem::size_of::<OpenDataStorageByDataIdIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct StorageReadWriteIn {
    pub offset: i64,
    pub size: u64,
}
const_assert_eq!(core::mem::size_of::<StorageReadWriteIn>(), 0x10);
