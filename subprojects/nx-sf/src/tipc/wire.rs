//! Wire-format types for TIPC message-type encoding.
//!
//! TIPC stores command IDs in the HIPC message type field. Request building and
//! response parsing live in the sibling `request` and `response` modules.

use crate::hipc;

/// Message type the first TIPC command id maps to.
///
/// Command ids are stored as `id + REQUEST_TYPE_BASE`, which keeps them clear
/// of the low message types CMIF occupies and of the close type at 15.
pub const REQUEST_TYPE_BASE: u16 = 16;

/// TIPC command types.
///
/// Unlike CMIF, TIPC encodes the command ID directly in the message type field
/// as `id + 16`. The `Close` variant is a special case with type = 15.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum CommandType {
    /// Close session (type = 15).
    Close = 15,
}

impl CommandType {
    /// Creates a request message type from a command ID.
    ///
    /// TIPC stores command ID in HIPC message type as ID + 16.
    #[inline]
    pub const fn request(id: u32) -> hipc::MessageType {
        // The message type field is 16 bits wide, so a command id an interface
        // can actually address always fits once the base is added.
        hipc::MessageType::from_raw((id + REQUEST_TYPE_BASE as u32) as u16)
    }
}

impl From<CommandType> for hipc::MessageType {
    fn from(cmd: CommandType) -> Self {
        hipc::MessageType::from_raw(cmd as u16)
    }
}
