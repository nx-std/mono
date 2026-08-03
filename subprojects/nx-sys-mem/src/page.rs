//! Page granularity, and addresses proven to sit on a page boundary.
//!
//! Every kernel memory object this crate wraps is page-granular: a transfer- or shared-memory
//! range the kernel accepts must start on a 4 KiB boundary, and one that does not is refused
//! with `InvalidAddress`. That requirement was previously spelled `& 0xFFF` at the boundary
//! that checks it and `0x1000` at the ones that rely on it, so the same rule appeared as two
//! unrelated literals and a checked address became indistinguishable from any other pointer
//! one line later.

use core::{ffi::c_void, ptr::NonNull};

/// The Horizon OS page size, in bytes.
pub const PAGE_SIZE: usize = 0x1000;

/// Low-order bits that are zero in a page-aligned address.
const PAGE_MASK: usize = PAGE_SIZE - 1;

/// A non-null address proven to sit on a page boundary.
///
/// Constructed only through [`TryFrom`], so holding one is proof the kernel will not refuse
/// the address for alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PageAlignedAddr(NonNull<c_void>);

impl PageAlignedAddr {
    /// Returns the address.
    #[inline]
    pub const fn to_ptr(self) -> NonNull<c_void> {
        self.0
    }
}

impl TryFrom<NonNull<c_void>> for PageAlignedAddr {
    type Error = UnalignedAddrError;

    /// Checks that `addr` sits on a page boundary.
    ///
    /// # Errors
    ///
    /// Returns [`UnalignedAddrError`] if it does not.
    fn try_from(addr: NonNull<c_void>) -> Result<Self, Self::Error> {
        let raw = addr.as_ptr() as usize;
        if raw & PAGE_MASK != 0 {
            return Err(UnalignedAddrError(raw));
        }
        Ok(Self(addr))
    }
}

/// An error indicating that an address does not sit on a page boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("The address {0:#x} is not page-aligned")]
pub struct UnalignedAddrError(usize);
