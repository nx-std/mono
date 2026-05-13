//! Wire-layout types for the NCM service.

use static_assertions::const_assert_eq;

/// Storage ID for content location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NcmStorageId {
    None = 0,
    Host = 1,
    GameCard = 2,
    BuiltInSystem = 3,
    BuiltInUser = 4,
    SdCard = 5,
    Any = 6,
}

/// Content type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NcmContentType {
    Meta = 0,
    Program = 1,
    Data = 2,
    Control = 3,
    HtmlDocument = 4,
    LegalInformation = 5,
    DeltaFragment = 6,
}

/// Content meta type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NcmContentMetaType {
    Unknown = 0x0,
    SystemProgram = 0x1,
    SystemData = 0x2,
    SystemUpdate = 0x3,
    BootImagePackage = 0x4,
    BootImagePackageSafe = 0x5,
    Application = 0x80,
    Patch = 0x81,
    AddOnContent = 0x82,
    Delta = 0x83,
    DataPatch = 0x84,
}

/// Content install type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NcmContentInstallType {
    Full = 0,
    FragmentOnly = 1,
    Unknown = 7,
}

/// Content meta platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NcmContentMetaPlatform {
    Nx = 0,
}

/// Content ID (16 bytes).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NcmContentId {
    pub c: [u8; 0x10],
}
const_assert_eq!(size_of::<NcmContentId>(), 0x10);

/// Placeholder ID (UUID, 16 bytes).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NcmPlaceHolderId {
    pub uuid: [u8; 0x10],
}
const_assert_eq!(size_of::<NcmPlaceHolderId>(), 0x10);

/// Content meta key.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NcmContentMetaKey {
    pub id: u64,
    pub version: u32,
    pub meta_type: u8,
    pub install_type: u8,
    pub padding: [u8; 2],
}
const_assert_eq!(size_of::<NcmContentMetaKey>(), 0x10);

/// Application content meta key.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NcmApplicationContentMetaKey {
    pub key: NcmContentMetaKey,
    pub application_id: u64,
}
const_assert_eq!(size_of::<NcmApplicationContentMetaKey>(), 0x18);

/// Content info.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NcmContentInfo {
    pub content_id: NcmContentId,
    pub size_low: u32,
    pub size_high: u8,
    pub attr: u8,
    pub content_type: u8,
    pub id_offset: u8,
}
const_assert_eq!(size_of::<NcmContentInfo>(), 0x18);

impl NcmContentInfo {
    /// Gets the content size as a `u64`.
    #[inline]
    pub fn size(&self) -> u64 {
        ((self.size_high as u64) << 32) | (self.size_low as u64)
    }

    /// Sets the content size from a `u64`.
    #[inline]
    pub fn set_size(&mut self, size: u64) {
        self.size_low = size as u32;
        self.size_high = (size >> 32) as u8;
    }
}

/// Packaged content info (hash + content info).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NcmPackagedContentInfo {
    pub hash: [u8; 0x20],
    pub info: NcmContentInfo,
}
const_assert_eq!(size_of::<NcmPackagedContentInfo>(), 0x38);

/// Content meta info.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NcmContentMetaInfo {
    pub id: u64,
    pub version: u32,
    pub meta_type: u8,
    pub attr: u8,
    pub padding: [u8; 2],
}
const_assert_eq!(size_of::<NcmContentMetaInfo>(), 0x10);

/// Content meta header.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NcmContentMetaHeader {
    pub extended_header_size: u16,
    pub content_count: u16,
    pub content_meta_count: u16,
    pub attributes: u8,
    pub storage_id: u8,
}
const_assert_eq!(size_of::<NcmContentMetaHeader>(), 0x8);

/// Application meta extended header.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NcmApplicationMetaExtendedHeader {
    pub patch_id: u64,
    pub required_system_version: u32,
    pub required_application_version: u32,
}
const_assert_eq!(size_of::<NcmApplicationMetaExtendedHeader>(), 0x10);

/// Patch meta extended header.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NcmPatchMetaExtendedHeader {
    pub application_id: u64,
    pub required_system_version: u32,
    pub extended_data_size: u32,
    pub reserved: [u8; 0x8],
}
const_assert_eq!(size_of::<NcmPatchMetaExtendedHeader>(), 0x18);

/// Add-on content meta extended header (15.0.0+).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NcmAddOnContentMetaExtendedHeader {
    pub application_id: u64,
    pub required_application_version: u32,
    pub content_accessibilities: u8,
    pub padding: [u8; 3],
    pub data_patch_id: u64,
}
const_assert_eq!(size_of::<NcmAddOnContentMetaExtendedHeader>(), 0x18);

/// Legacy add-on content meta extended header (1.0.0–14.1.2).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NcmLegacyAddOnContentMetaExtendedHeader {
    pub application_id: u64,
    pub required_application_version: u32,
    pub padding: u32,
}
const_assert_eq!(size_of::<NcmLegacyAddOnContentMetaExtendedHeader>(), 0x10);

/// Data patch meta extended header.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NcmDataPatchMetaExtendedHeader {
    pub data_id: u64,
    pub application_id: u64,
    pub required_application_version: u32,
    pub extended_data_size: u32,
    pub padding: u64,
}
const_assert_eq!(size_of::<NcmDataPatchMetaExtendedHeader>(), 0x20);

/// System update meta extended header.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NcmSystemUpdateMetaExtendedHeader {
    pub extended_data_size: u32,
}
const_assert_eq!(size_of::<NcmSystemUpdateMetaExtendedHeader>(), 0x4);

/// Program location.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NcmProgramLocation {
    pub program_id: u64,
    pub storage_id: u8,
    pub pad: [u8; 7],
}
const_assert_eq!(size_of::<NcmProgramLocation>(), 0x10);

/// Rights ID (used with content storage).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NcmRightsId {
    pub rights_id: [u8; 0x10],
    pub key_generation: u8,
    pub pad: [u8; 7],
}
const_assert_eq!(size_of::<NcmRightsId>(), 0x18);

/// Content attributes for filesystem operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FsContentAttributes {
    None = 0x0,
    All = 0xF,
}

/// Maximum filesystem path length (0x301 bytes).
pub const FS_MAX_PATH: usize = 0x301;

// ---------------------------------------------------------------------------
// IPC wire-layout input structs
// ---------------------------------------------------------------------------

/// Input for CreatePlaceHolder (pre-16.0.0): content_id, placeholder_id, size.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct CreatePlaceHolderLegacyIn {
    pub content_id: NcmContentId,
    pub placeholder_id: NcmPlaceHolderId,
    pub size: i64,
}
const_assert_eq!(size_of::<CreatePlaceHolderLegacyIn>(), 0x28);

/// Input for CreatePlaceHolder (16.0.0+): placeholder_id, content_id, size.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct CreatePlaceHolderIn {
    pub placeholder_id: NcmPlaceHolderId,
    pub content_id: NcmContentId,
    pub size: i64,
}
const_assert_eq!(size_of::<CreatePlaceHolderIn>(), 0x28);

/// Input for WritePlaceHolder: placeholder_id, offset.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct WritePlaceHolderIn {
    pub placeholder_id: NcmPlaceHolderId,
    pub offset: u64,
}
const_assert_eq!(size_of::<WritePlaceHolderIn>(), 0x18);

/// Input for Register (pre-16.0.0): content_id, placeholder_id.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct RegisterLegacyIn {
    pub content_id: NcmContentId,
    pub placeholder_id: NcmPlaceHolderId,
}
const_assert_eq!(size_of::<RegisterLegacyIn>(), 0x20);

/// Input for Register (16.0.0+): placeholder_id, content_id.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct RegisterIn {
    pub placeholder_id: NcmPlaceHolderId,
    pub content_id: NcmContentId,
}
const_assert_eq!(size_of::<RegisterIn>(), 0x20);

/// Input for RevertToPlaceHolder (pre-16.0.0): old, new, placeholder.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct RevertToPlaceHolderLegacyIn {
    pub old_content_id: NcmContentId,
    pub new_content_id: NcmContentId,
    pub placeholder_id: NcmPlaceHolderId,
}
const_assert_eq!(size_of::<RevertToPlaceHolderLegacyIn>(), 0x30);

/// Input for RevertToPlaceHolder (16.0.0+): placeholder, old, new.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct RevertToPlaceHolderIn {
    pub placeholder_id: NcmPlaceHolderId,
    pub old_content_id: NcmContentId,
    pub new_content_id: NcmContentId,
}
const_assert_eq!(size_of::<RevertToPlaceHolderIn>(), 0x30);

/// Input for SetPlaceHolderSize: placeholder_id, size.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct SetPlaceHolderSizeIn {
    pub placeholder_id: NcmPlaceHolderId,
    pub size: i64,
}
const_assert_eq!(size_of::<SetPlaceHolderSizeIn>(), 0x18);

/// Input for ReadContentIdFile: content_id, offset.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ReadContentIdFileIn {
    pub content_id: NcmContentId,
    pub offset: i64,
}
const_assert_eq!(size_of::<ReadContentIdFileIn>(), 0x18);

/// Input for GetRightsIdFromPlaceHolderId: placeholder_id, attr.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetRightsIdFromPlaceHolderIdIn {
    pub placeholder_id: NcmPlaceHolderId,
    pub attr: u8,
}
const_assert_eq!(size_of::<GetRightsIdFromPlaceHolderIdIn>(), 0x11);

/// Input for GetRightsIdFromContentId: content_id, attr.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetRightsIdFromContentIdIn {
    pub content_id: NcmContentId,
    pub attr: u8,
}
const_assert_eq!(size_of::<GetRightsIdFromContentIdIn>(), 0x11);

/// Input for WriteContentForDebug: content_id, offset.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct WriteContentForDebugIn {
    pub content_id: NcmContentId,
    pub offset: i64,
}
const_assert_eq!(size_of::<WriteContentForDebugIn>(), 0x18);

/// Input for GetRightsIdFromPlaceHolderIdWithCache (pre-16.0.0).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetRightsIdWithCacheLegacyIn {
    pub cache_content_id: NcmContentId,
    pub placeholder_id: NcmPlaceHolderId,
}
const_assert_eq!(size_of::<GetRightsIdWithCacheLegacyIn>(), 0x20);

/// Input for GetRightsIdFromPlaceHolderIdWithCache (16.0.0+).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetRightsIdWithCacheIn {
    pub placeholder_id: NcmPlaceHolderId,
    pub cache_content_id: NcmContentId,
    pub attr: u8,
}
const_assert_eq!(size_of::<GetRightsIdWithCacheIn>(), 0x21);

/// Input for GetProgramId (17.0.0+): content_id, attr.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetProgramIdIn {
    pub content_id: NcmContentId,
    pub attr: u8,
}
const_assert_eq!(size_of::<GetProgramIdIn>(), 0x11);

// ---------------------------------------------------------------------------
// IContentMetaDatabase wire-layout input structs
// ---------------------------------------------------------------------------

/// Input for GetContentIdByType: type, padding, key.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetContentIdByTypeIn {
    pub content_type: u8,
    pub padding: [u8; 7],
    pub key: NcmContentMetaKey,
}
const_assert_eq!(size_of::<GetContentIdByTypeIn>(), 0x18);

/// Input for ListContentInfo / ListContentMetaInfo: start_index, pad, key.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ListContentInfoIn {
    pub start_index: i32,
    pub pad: u32,
    pub key: NcmContentMetaKey,
}
const_assert_eq!(size_of::<ListContentInfoIn>(), 0x18);

/// Input for List (cmd 5): meta_type, install_type, padding, id, id_min, id_max.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ListIn {
    pub meta_type: u8,
    pub install_type: u8,
    pub padding: [u8; 6],
    pub id: u64,
    pub id_min: u64,
    pub id_max: u64,
}
const_assert_eq!(size_of::<ListIn>(), 0x20);

/// Output for List / ListApplication: entries_total, entries_written.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ListOut {
    pub entries_total: i32,
    pub entries_written: i32,
}
const_assert_eq!(size_of::<ListOut>(), 0x8);

/// Input for HasContent: content_id, key.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct HasContentIn {
    pub content_id: NcmContentId,
    pub key: NcmContentMetaKey,
}
const_assert_eq!(size_of::<HasContentIn>(), 0x20);

/// Input for GetContentIdByTypeAndIdOffset: type, id_offset, padding, key.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetContentIdByTypeAndIdOffsetIn {
    pub content_type: u8,
    pub id_offset: u8,
    pub padding: [u8; 6],
    pub key: NcmContentMetaKey,
}
const_assert_eq!(size_of::<GetContentIdByTypeAndIdOffsetIn>(), 0x18);
