//! Nanosecond type and constants

// Code borrowed, with modifications, from: https://github.com/rust-lang/rust/blob/ed49386d3aa3a445a9889707fd405df01723eced/library/core/src/num/niche_types.rs#L112
// Licensed under: Apache-2.0 OR MIT

use core::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
};

use static_assertions::const_assert_eq;

/// The number of nanoseconds in a second.
pub const NSEC_PER_SEC: i64 = 1_000_000_000;

/// The minimum valid value for a nanosecond.
pub const NSEC_MIN: i64 = 0;

/// The maximum valid value for a nanosecond.
pub const NSEC_MAX: i64 = NSEC_PER_SEC - 1;

/// A type representing a count of nanoseconds, stored as a `u32`.
///
/// This type is used to represent sub-second time intervals with nanosecond precision.
/// Valid values are in the range `0..=999_999_999` (i.e. 0 to just under 1 second).
///
/// The type is `#[repr(transparent)]` meaning it has the same ABI layout as its inner `u32`.
///
/// # Safety
///
/// Creating invalid values (outside the valid range) is unsafe and results in undefined behavior.
/// Use the safe constructor [`try_from`](Nanoseconds::try_from) to create values safely.
#[derive(Clone, Copy, Eq)]
#[repr(transparent)]
pub struct Nanoseconds(u32);

// Asserts that the size of the type is the same as the size of the inner type
const_assert_eq!(size_of::<Nanoseconds>(), size_of::<u32>());

impl Nanoseconds {
    /// The zero value for this type.
    // SAFETY: 0 is within the valid range
    pub const ZERO: Self = unsafe { Nanoseconds::new_unchecked(0) };

    /// Constructs an instance of this type from the underlying integer
    /// primitive without checking if the value is within the valid range.
    ///
    /// # Safety
    ///
    /// Immediate language UB if the value is not within the valid range,
    /// i.e., if it is out of the range `0..=999_999_999`.
    #[inline]
    pub const unsafe fn new_unchecked(val: u32) -> Self {
        Nanoseconds(val)
    }

    /// Constructs an instance of this type from the underlying integer
    #[inline]
    pub const fn as_inner(self) -> u32 {
        unsafe { core::mem::transmute(self) }
    }
}

impl Default for Nanoseconds {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

impl TryFrom<i64> for Nanoseconds {
    type Error = OutOfRangeError<i64>;

    /// Try converting from an `i64` to a `Nanoseconds` value.
    ///
    /// If the value is not within the valid range, an [`OutOfRangeError`] is returned.
    #[inline]
    fn try_from(val: i64) -> Result<Self, Self::Error> {
        if (NSEC_MIN..=NSEC_MAX).contains(&val) {
            Ok(unsafe { Nanoseconds::new_unchecked(val as u32) })
        } else {
            Err(OutOfRangeError(val))
        }
    }
}

impl TryFrom<u32> for Nanoseconds {
    type Error = OutOfRangeError<u32>;

    /// Try converting from a `u32` to a `Nanoseconds` value.
    ///
    /// If the value is not within the valid range, an [`OutOfRangeError`] is returned.
    #[inline]
    fn try_from(val: u32) -> Result<Self, Self::Error> {
        if val <= NSEC_MAX as u32 {
            Ok(unsafe { Nanoseconds::new_unchecked(val) })
        } else {
            Err(OutOfRangeError(val))
        }
    }
}

impl PartialEq for Nanoseconds {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_inner() == other.as_inner()
    }
}

impl Ord for Nanoseconds {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&self.as_inner(), &other.as_inner())
    }
}

impl PartialOrd for Nanoseconds {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Hash for Nanoseconds {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.as_inner(), state);
    }
}

impl fmt::Display for Nanoseconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <u32 as fmt::Display>::fmt(&self.as_inner(), f)
    }
}

impl fmt::Debug for Nanoseconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <u32 as fmt::Debug>::fmt(&self.as_inner(), f)
    }
}

/// An error indicating that a value is out of range
#[derive(Debug, thiserror::Error)]
#[error("value out of range: {0}")]
pub struct OutOfRangeError<T: fmt::Debug>(T);
