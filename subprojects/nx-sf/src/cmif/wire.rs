//! Wire-format types and prefix decoding for CMIF.
//!
//! The fixed-layout structs in this module describe the on-the-wire byte
//! layouts used by CMIF. Higher-level request/response logic lives in the
//! sibling `request` and `response` modules.

use static_assertions::const_assert_eq;
use zerocopy::{IntoBytes, KnownLayout};

use crate::hipc;

/// Magic number for CMIF input headers ("SFCI" - Service Framework Command Input).
pub const IN_HEADER_MAGIC: u32 = 0x49434653;

/// Magic number for CMIF output headers ("SFCO" - Service Framework Command Output).
pub const OUT_HEADER_MAGIC: u32 = 0x4F434653;

/// Maximum number of domain objects passed in a single CMIF request.
pub const CMIF_MAX_OBJECTS: usize = 8;

/// CMIF command type (stored in HIPC message type field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CommandType {
    /// Invalid command.
    Invalid = 0,
    /// Legacy request (pre-5.0.0).
    LegacyRequest = 1,
    /// Close session.
    Close = 2,
    /// Legacy control request.
    LegacyControl = 3,
    /// Standard request.
    Request = 4,
    /// Control request (domain conversion, cloning, etc.).
    Control = 5,
    /// Request with context token (5.0.0+).
    RequestWithContext = 6,
    /// Control request with context token.
    ControlWithContext = 7,
}

impl From<CommandType> for hipc::MessageType {
    fn from(cmd: CommandType) -> Self {
        hipc::MessageType::from_raw(cmd as u16)
    }
}

/// Domain request type (stored in domain header).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DomainRequestType {
    /// Invalid request.
    Invalid = 0,
    /// Send message to domain object.
    SendMessage = 1,
    /// Close domain object.
    Close = 2,
}

/// CMIF input header (16 bytes).
#[derive(
    Debug, Clone, Copy, Default, zerocopy::FromBytes, IntoBytes, zerocopy::Immutable, KnownLayout,
)]
#[repr(C)]
pub struct InHeader {
    /// Magic number (`"SFCI"` = 0x49434653).
    pub magic: u32,
    /// Protocol version (0 = standard, 1 = with context).
    pub version: u32,
    /// Command/method ID to invoke.
    pub command_id: u32,
    /// Context token for versioning (non-domain only).
    pub token: u32,
}

const_assert_eq!(size_of::<InHeader>(), 16);

/// CMIF output header (16 bytes).
#[derive(
    Debug, Clone, Copy, Default, zerocopy::FromBytes, IntoBytes, zerocopy::Immutable, KnownLayout,
)]
#[repr(C)]
pub struct OutHeader {
    /// Magic number (`"SFCO"` = 0x4F434653).
    pub magic: u32,
    /// Protocol version.
    pub version: u32,
    /// Result code (0 = success).
    pub result: u32,
    /// Echo of request token.
    pub token: u32,
}

const_assert_eq!(size_of::<OutHeader>(), 16);

/// Domain input header (16 bytes).
#[derive(
    Debug, Clone, Copy, Default, zerocopy::FromBytes, IntoBytes, zerocopy::Immutable, KnownLayout,
)]
#[repr(C)]
pub struct DomainInHeader {
    /// Request type (SendMessage or Close).
    pub request_type: u8,
    /// Number of object IDs in request.
    pub num_in_objects: u8,
    /// Size of CMIF header + payload.
    pub data_size: u16,
    /// Target object ID within domain.
    pub object_id: u32,
    /// Reserved padding.
    pub _padding: u32,
    /// Context token.
    pub token: u32,
}

const_assert_eq!(size_of::<DomainInHeader>(), 16);

/// Domain output header (16 bytes).
#[derive(
    Debug, Clone, Copy, Default, zerocopy::FromBytes, IntoBytes, zerocopy::Immutable, KnownLayout,
)]
#[repr(C)]
pub struct DomainOutHeader {
    /// Request type (SendMessage or Close).
    pub request_type: u8,
    /// Number of object IDs in response.
    pub num_out_objects: u8,
    /// Reserved padding.
    pub _padding: u16,
    /// Echo of the request's data size.
    pub data_size: u32,
    /// Echo of the request's object ID.
    pub object_id: u32,
    /// Context token.
    pub token: u32,
}

const_assert_eq!(size_of::<DomainOutHeader>(), 16);
