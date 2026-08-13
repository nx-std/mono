//! The method identifier a request names.

/// Identifier of the method a request invokes on an interface.
///
/// The counterpart of `http::Method`: the part of the head a router keys on.
/// Both protocols carry it, though they store it in different places - CMIF in
/// its in-band header, TIPC in the message type - and this type is what they
/// agree on once decoded.
///
/// Interfaces assign the numbers themselves and no value is reserved, so every
/// `u32` is a well-formed command id. The newtype earns its keep by not being
/// interchangeable with the result codes, tokens, and object ids a request head
/// is otherwise full of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandId(u32);

impl CommandId {
    /// Wraps a raw command id.
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the raw id.
    #[inline]
    pub const fn to_raw(self) -> u32 {
        self.0
    }
}

impl From<u32> for CommandId {
    #[inline]
    fn from(id: u32) -> Self {
        Self::new(id)
    }
}

impl From<CommandId> for u32 {
    #[inline]
    fn from(id: CommandId) -> Self {
        id.to_raw()
    }
}
