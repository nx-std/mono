//! Parental controls wire-layout types.

use static_assertions::const_assert_eq;

/// Parental controls restriction settings returned by the service.
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct PctlRestrictionSettings {
    pub rating_age: u8,
    pub sns_post_restriction: u8,
    pub free_communication_restriction: u8,
}

const_assert_eq!(size_of::<PctlRestrictionSettings>(), 0x3);
