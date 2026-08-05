//! The rights id a content path is signed under, and the key generation that
//! goes with it.

use static_assertions::const_assert_eq;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RightsId {
    pub c: [u8; 0x10],
}
const_assert_eq!(core::mem::size_of::<RightsId>(), 0x10);

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetRightsIdAndKeyGenOut {
    pub key_generation: u8,
    pub padding: [u8; 7],
    pub rights_id: RightsId,
}
const_assert_eq!(core::mem::size_of::<GetRightsIdAndKeyGenOut>(), 0x18);
