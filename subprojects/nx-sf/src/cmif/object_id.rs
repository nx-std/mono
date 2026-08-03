//! CMIF domain object identifiers.

/// CMIF object identifier.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(transparent)]
pub struct ObjectId(u32);

impl ObjectId {
    /// Creates an object identifier when the raw value is non-zero.
    pub fn new(raw: u32) -> Option<Self> {
        (raw != 0).then_some(Self(raw))
    }

    /// Wraps a raw object identifier without checking it.
    ///
    /// The caller must ensure `raw != 0`, since zero names no domain object. A zero
    /// identifier is not undefined behaviour: this type derives [`zerocopy::FromBytes`],
    /// which declares every byte pattern a valid instance, so the value simply reaches the
    /// server and is rejected there.
    pub const fn from_raw_unchecked(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw object identifier.
    pub fn to_raw(self) -> u32 {
        self.0
    }
}
