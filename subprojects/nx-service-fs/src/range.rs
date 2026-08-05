//! Operating on a byte range, which a file and a storage answer alike.
//!
//! This is the one vocabulary [`crate::file`] and [`crate::storage`] share.
//! It sits beside them rather than inside either so that neither has to reach
//! into the other for it.

use static_assertions::const_assert_eq;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OperationId {
    Clear = 0,
    ClearSignature = 1,
    InvalidateCache = 2,
    QueryRange = 3,
}

#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct RangeInfo {
    pub aes_ctr_key_type: u32,
    pub speed_emulation_type: u32,
    pub reserved: [u32; 0x38 / 4],
}
const_assert_eq!(core::mem::size_of::<RangeInfo>(), 0x40);

#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct OperateRangeIn {
    pub op_id: u32,
    pub pad: u32,
    pub off: i64,
    pub len: i64,
}
const_assert_eq!(core::mem::size_of::<OperateRangeIn>(), 0x18);
