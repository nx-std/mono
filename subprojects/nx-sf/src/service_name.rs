//! Service name type for Horizon OS services.
//!
//! Service names in Horizon OS are up to 8 characters, with remaining
//! bytes set to zero. Since most service names are short (e.g., "sm:", "fsp-srv"),
//! the entire name fits in a single `u64` register. This allows efficient
//! comparison and passing without heap allocation or pointer indirection.

use static_assertions::const_assert_eq;

/// Fixed-capacity ASCII string for service names (max 8 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct ServiceName {
    name: [u8; 8],
}

const_assert_eq!(size_of::<ServiceName>(), size_of::<u64>());

impl ServiceName {
    /// Maximum length of a service name (8 characters).
    pub const MAX_LEN: usize = 8;

    /// Creates a service name from a string slice.
    ///
    /// Returns `None` if the name exceeds 8 characters.
    ///
    /// # Panics
    ///
    /// Panics if the name contains non-ASCII characters.
    #[inline]
    pub const fn new(name: &str) -> Option<Self> {
        let bytes = name.as_bytes();
        if bytes.len() > Self::MAX_LEN {
            return None;
        }

        let mut result = [0u8; 8];
        let mut c = 0;
        while c < bytes.len() {
            assert!(bytes[c].is_ascii(), "service name must be ASCII");
            result[c] = bytes[c];
            c += 1;
        }
        Some(Self { name: result })
    }

    /// Creates a service name from a string slice, truncating if needed.
    ///
    /// If the name is longer than 8 characters, only the first 8 are used.
    ///
    /// # Panics
    ///
    /// Panics if the name contains non-ASCII characters.
    #[inline]
    pub const fn new_truncate(name: &str) -> Self {
        let bytes = name.as_bytes();
        let len = if bytes.len() > Self::MAX_LEN {
            Self::MAX_LEN
        } else {
            bytes.len()
        };

        let mut result = [0u8; 8];
        let mut c = 0;
        while c < len {
            assert!(bytes[c].is_ascii(), "service name must be ASCII");
            result[c] = bytes[c];
            c += 1;
        }

        Self { name: result }
    }

    /// Creates a service name from raw bytes.
    ///
    /// # Safety
    ///
    /// Caller must ensure all non-zero bytes are valid ASCII.
    #[inline]
    pub const unsafe fn from_bytes(bytes: [u8; 8]) -> Self {
        Self { name: bytes }
    }

    /// Creates a service name from a `u64`.
    ///
    /// # Safety
    ///
    /// Caller must ensure all non-zero bytes are valid ASCII.
    #[inline]
    pub const unsafe fn from_u64(value: u64) -> Self {
        Self {
            name: value.to_le_bytes(),
        }
    }

    /// Converts the service name to a `u64` for efficient comparison.
    #[inline]
    pub const fn to_u64(&self) -> u64 {
        u64::from_le_bytes(self.name)
    }

    /// Alias for [`to_u64`](Self::to_u64).
    #[inline]
    pub const fn to_raw(&self) -> u64 {
        self.to_u64()
    }

    /// Returns the bytes of the service name (excluding trailing zeros).
    #[inline]
    pub const fn as_bytes(&self) -> &[u8] {
        let name_ptr = self.name.as_ptr();
        let name_len = self.len();
        // SAFETY: name_ptr is valid for name_len bytes, which is <= 8.
        unsafe { core::slice::from_raw_parts(name_ptr, name_len) }
    }

    /// Returns the full 8-byte buffer including null padding.
    #[inline]
    pub const fn as_bytes_raw(&self) -> &[u8; 8] {
        &self.name
    }

    /// Returns the service name as a raw pointer to the underlying bytes.
    ///
    /// The returned pointer is valid for the lifetime of `self`.
    #[inline]
    pub const fn as_ptr(&self) -> *const u8 {
        self.name.as_ptr().cast()
    }

    /// Returns the length of the service name (excluding trailing zeros).
    pub const fn len(&self) -> usize {
        let mut i = 0;
        while i < Self::MAX_LEN {
            if self.name[i] == 0 {
                return i;
            }
            i += 1;
        }
        Self::MAX_LEN
    }

    /// Returns whether the service name is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.name[0] == 0
    }

    /// Returns the service name as a string slice.
    ///
    /// Service names are ASCII, so this conversion is infallible.
    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: Service names are always ASCII (subset of UTF-8).
        unsafe { core::str::from_utf8_unchecked(self.as_bytes()) }
    }
}

impl core::fmt::Display for ServiceName {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl PartialEq<str> for ServiceName {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<ServiceName> for str {
    #[inline]
    fn eq(&self, other: &ServiceName) -> bool {
        other == self
    }
}

impl PartialEq<&str> for ServiceName {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<ServiceName> for &str {
    #[inline]
    fn eq(&self, other: &ServiceName) -> bool {
        other == self
    }
}

impl PartialEq<core::ffi::CStr> for ServiceName {
    #[inline]
    fn eq(&self, other: &core::ffi::CStr) -> bool {
        self.as_bytes() == other.to_bytes()
    }
}

impl PartialEq<ServiceName> for core::ffi::CStr {
    #[inline]
    fn eq(&self, other: &ServiceName) -> bool {
        other == self
    }
}

// PartialEq implementations for numeric/raw types

impl PartialEq<u64> for ServiceName {
    #[inline]
    fn eq(&self, other: &u64) -> bool {
        self.to_u64() == *other
    }
}

impl PartialEq<ServiceName> for u64 {
    #[inline]
    fn eq(&self, other: &ServiceName) -> bool {
        other == self
    }
}

// PartialEq implementations for byte slices

impl PartialEq<[u8]> for ServiceName {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.as_bytes() == other
    }
}

impl PartialEq<ServiceName> for [u8] {
    #[inline]
    fn eq(&self, other: &ServiceName) -> bool {
        other == self
    }
}

impl PartialEq<&[u8]> for ServiceName {
    #[inline]
    fn eq(&self, other: &&[u8]) -> bool {
        self.as_bytes() == *other
    }
}

impl PartialEq<ServiceName> for &[u8] {
    #[inline]
    fn eq(&self, other: &ServiceName) -> bool {
        other == self
    }
}

impl PartialEq<[u8; 8]> for ServiceName {
    #[inline]
    fn eq(&self, other: &[u8; 8]) -> bool {
        self.as_bytes_raw() == other
    }
}

impl PartialEq<ServiceName> for [u8; 8] {
    #[inline]
    fn eq(&self, other: &ServiceName) -> bool {
        other == self
    }
}

impl PartialEq<&[u8; 8]> for ServiceName {
    #[inline]
    fn eq(&self, other: &&[u8; 8]) -> bool {
        self.as_bytes_raw() == *other
    }
}

impl PartialEq<ServiceName> for &[u8; 8] {
    #[inline]
    fn eq(&self, other: &ServiceName) -> bool {
        other == self
    }
}
