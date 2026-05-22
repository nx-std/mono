//! Wire-layout types for the PL service.

/// Shared font type identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SharedFontType {
    /// Japan, US and Europe (standard Latin/CJK).
    Standard = 0,
    /// Chinese Simplified.
    ChineseSimplified = 1,
    /// Extended Chinese Simplified.
    ExtChineseSimplified = 2,
    /// Chinese Traditional.
    ChineseTraditional = 3,
    /// Korean (Hangul).
    Ko = 4,
    /// Nintendo Extended (special Nintendo-specific characters).
    NintendoExt = 5,
}

impl SharedFontType {
    /// Total number of shared font types.
    pub const TOTAL: usize = 6;
}

/// Service type selection for `pl:u` vs `pl:s`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlServiceType {
    /// User interface (`pl:u`).
    User,
    /// System interface (`pl:s`).
    System,
}

/// Output from [`get_shared_font`](crate::PlService::get_shared_font).
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct GetSharedFontOut {
    /// Whether fonts have finished loading (non-zero = loaded).
    pub fonts_loaded: u8,
    /// Padding.
    pub _pad: [u8; 3],
    /// Total number of fonts returned.
    pub total_fonts: i32,
}
