//! NV driver file descriptor type.

/// NV driver file descriptor - identifies an opened device.
///
/// Returned by [`NvService::open()`](crate::NvService::open) and passed to
/// ioctl/close operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Fd(u32);

impl Fd {
    /// Creates a new file descriptor from a raw value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `raw` is a valid NV driver file descriptor
    /// returned by a previous call to [`NvService::open()`](crate::NvService::open).
    #[inline]
    pub const unsafe fn new_unchecked(raw: u32) -> Self {
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
