//! The access log `fsp-srv` keeps on behalf of a process.

use static_assertions::const_assert_eq;

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ProgramIndexForAccessLogOut {
    pub index: u32,
    pub count: u32,
}
const_assert_eq!(core::mem::size_of::<ProgramIndexForAccessLogOut>(), 0x8);
