//! NV driver file descriptor type.

/// NV driver file descriptor - identifies an opened device.
///
/// Returned by [`NvService::open()`](crate::NvService::open) and passed to
/// ioctl/close operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Fd(u32);

impl Fd {
    /// Wraps a raw file descriptor without checking it.
    ///
    /// The caller must ensure `raw` was returned by a previous
    /// [`NvService::open()`](crate::NvService::open) and not since closed. The driver owns the
    /// descriptor table, so nothing here can check that; an unknown descriptor is answered
    /// with an NV error rather than faulting, which is why this is a safe function.
    #[inline]
    pub const fn from_raw_unchecked(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw u32 value for FFI/IPC calls.
    #[inline]
    pub const fn to_raw(self) -> u32 {
        self.0
    }
}

impl From<Fd> for u32 {
    #[inline]
    fn from(fd: Fd) -> Self {
        fd.0
    }
}
