//! Save data: the kinds one is keyed by, the attribute that names one, and the
//! request payloads the save-data commands send.
//!
//! [`AccountUid`] lives here rather than beside the other identity types because
//! save data is the only thing in this crate keyed by a user: keeping it here is
//! what lets this module stand alone, referring to no sibling and referred to by
//! none.

use static_assertions::const_assert_eq;

/// Application id naming the running application's own save data.
pub const FS_SAVEDATA_CURRENT_APPLICATIONID: u64 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum SaveDataSpaceId {
    System = 0,
    User = 1,
    SdSystem = 2,
    Temporary = 3,
    SdUser = 4,
    ProperSystem = 100,
    SafeMode = 101,
    All = -1,
}

impl TryFrom<i32> for SaveDataSpaceId {
    type Error = UnknownSaveDataSpaceId;

    /// Decodes the space id a C caller passes as the C enum's `int`.
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::System),
            1 => Ok(Self::User),
            2 => Ok(Self::SdSystem),
            3 => Ok(Self::Temporary),
            4 => Ok(Self::SdUser),
            100 => Ok(Self::ProperSystem),
            101 => Ok(Self::SafeMode),
            -1 => Ok(Self::All),
            _ => Err(UnknownSaveDataSpaceId(value)),
        }
    }
}

/// Error returned by [`SaveDataSpaceId::try_from`].
///
/// The value names no space the server knows, so no request was sent.
#[derive(Debug, thiserror::Error)]
#[error("unknown save data space id: {0}")]
pub struct UnknownSaveDataSpaceId(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SaveDataType {
    System = 0,
    Account = 1,
    Bcat = 2,
    Device = 3,
    Temporary = 4,
    Cache = 5,
    SystemBcat = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SaveDataRank {
    Primary = 0,
    Secondary = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SaveDataMetaType {
    None = 0,
    Thumbnail = 1,
    ExtensionContext = 2,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SaveDataFlags: u32 {
        const KEEP_AFTER_RESETTING_SYSTEM_SAVE_DATA = 1 << 0;
        const KEEP_AFTER_REFURBISHMENT = 1 << 1;
        const KEEP_AFTER_RESETTING_SYSTEM_SAVE_DATA_WITHOUT_USER_SAVE_DATA = 1 << 2;
        const NEEDS_SECURE_DELETE = 1 << 3;
    }
}

/// Account user id. All-zero names the common save data rather than a user's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct AccountUid {
    pub uid: [u64; 2],
}
const_assert_eq!(core::mem::size_of::<AccountUid>(), 0x10);

/// Names the save data a command addresses.
///
/// Only the fields a given save-data kind is keyed by are filled; the rest stay
/// zero, which is what the server expects and what the shaped openers on
/// [`crate::FsService`] rely on `Default` for.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct SaveDataAttribute {
    pub application_id: u64,
    pub uid: AccountUid,
    pub system_save_data_id: u64,
    pub save_data_type: u8,
    pub save_data_rank: u8,
    pub save_data_index: u16,
    pub pad_x24: u32,
    pub unk_x28: u64,
    pub unk_x30: u64,
    pub unk_x38: u64,
}
const_assert_eq!(core::mem::size_of::<SaveDataAttribute>(), 0x40);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SaveDataExtraData {
    pub attr: SaveDataAttribute,
    pub owner_id: u64,
    pub timestamp: u64,
    pub flags: u32,
    pub unk_x54: u32,
    pub data_size: i64,
    pub journal_size: i64,
    pub commit_id: u64,
    pub unused: [u8; 0x190],
}
const_assert_eq!(core::mem::size_of::<SaveDataExtraData>(), 0x200);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SaveDataMetaInfo {
    pub size: u32,
    pub meta_type: u8,
    pub reserved: [u8; 0x0B],
}
const_assert_eq!(core::mem::size_of::<SaveDataMetaInfo>(), 0x10);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SaveDataCreationInfo {
    pub save_data_size: i64,
    pub journal_size: i64,
    pub available_size: u64,
    pub owner_id: u64,
    pub flags: u32,
    pub save_data_space_id: u8,
    pub unk: u8,
    pub padding: [u8; 0x1a],
}
const_assert_eq!(core::mem::size_of::<SaveDataCreationInfo>(), 0x40);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SaveDataInfo {
    pub save_data_id: u64,
    pub save_data_space_id: u8,
    pub save_data_type: u8,
    pub pad: [u8; 6],
    pub uid: AccountUid,
    pub system_save_data_id: u64,
    pub application_id: u64,
    pub size: u64,
    pub save_data_index: u16,
    pub save_data_rank: u8,
    pub unk_x3b: [u8; 0x25],
}
const_assert_eq!(core::mem::size_of::<SaveDataInfo>(), 0x60);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SaveDataFilter {
    pub filter_by_application_id: u8,
    pub filter_by_save_data_type: u8,
    pub filter_by_user_id: u8,
    pub filter_by_system_save_data_id: u8,
    pub filter_by_index: u8,
    pub save_data_rank: u8,
    pub padding: [u8; 2],
    pub attr: SaveDataAttribute,
}
const_assert_eq!(core::mem::size_of::<SaveDataFilter>(), 0x48);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct DeleteSaveDataBySpaceIdIn {
    pub save_data_space_id: u8,
    pub _pad: [u8; 7],
    pub save_id: u64,
}
const_assert_eq!(core::mem::size_of::<DeleteSaveDataBySpaceIdIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct DeleteSaveDataByAttributeIn {
    pub save_data_space_id: u8,
    pub _pad: [u8; 7],
    pub attr: SaveDataAttribute,
}
const_assert_eq!(core::mem::size_of::<DeleteSaveDataByAttributeIn>(), 0x48);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct CreateSaveDataIn {
    pub attr: SaveDataAttribute,
    pub creation_info: SaveDataCreationInfo,
    pub meta: SaveDataMetaInfo,
}
const_assert_eq!(core::mem::size_of::<CreateSaveDataIn>(), 0x90);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct CreateSaveDataBySystemIdIn {
    pub attr: SaveDataAttribute,
    pub creation_info: SaveDataCreationInfo,
}
const_assert_eq!(core::mem::size_of::<CreateSaveDataBySystemIdIn>(), 0x80);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ExtendSaveDataIn {
    pub save_data_space_id: u8,
    pub pad: [u8; 7],
    pub save_id: u64,
    pub data_size: i64,
    pub journal_size: i64,
}
const_assert_eq!(core::mem::size_of::<ExtendSaveDataIn>(), 0x20);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenSaveDataIn {
    pub save_data_space_id: u8,
    pub pad: [u8; 7],
    pub attr: SaveDataAttribute,
}
const_assert_eq!(core::mem::size_of::<OpenSaveDataIn>(), 0x48);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ReadExtraDataBySpaceIdIn {
    pub save_data_space_id: u8,
    pub _pad: [u8; 7],
    pub save_id: u64,
}
const_assert_eq!(core::mem::size_of::<ReadExtraDataBySpaceIdIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct WriteExtraDataIn {
    pub save_data_space_id: u8,
    pub _pad: [u8; 7],
    pub save_id: u64,
}
const_assert_eq!(core::mem::size_of::<WriteExtraDataIn>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenSaveDataInfoReaderWithFilterIn {
    pub save_data_space_id: u8,
    pub pad: [u8; 7],
    pub filter: SaveDataFilter,
}
const_assert_eq!(
    core::mem::size_of::<OpenSaveDataInfoReaderWithFilterIn>(),
    0x50
);
