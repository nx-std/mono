//! CMIF domain object identifiers.

/// CMIF object identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectId(u32);

impl ObjectId {
    /// Creates an object identifier when the raw value is non-zero.
    pub fn new(raw: u32) -> Option<Self> {
        (raw != 0).then_some(Self(raw))
    }

    /// Creates an object identifier without validating the raw value.
    ///
    /// # Safety
    ///
    /// The caller must ensure `raw != 0`.
    pub unsafe fn new_unchecked(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw object identifier.
    pub fn to_raw(self) -> u32 {
        self.0
    }
}
