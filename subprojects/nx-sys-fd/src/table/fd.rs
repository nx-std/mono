//! The descriptor number, and the bound on it.

/// Number of descriptors the table can hold.
///
/// Matches the capacity the C standard library used, so a program that ran against the C table
/// cannot run out of descriptors sooner here.
pub const MAX_FD: usize = 1024;

/// An open descriptor.
///
/// A value of this type names a descriptor slot that exists; whether it is open is a separate
/// question the table answers.
///
/// Validation lives in the [`TryFrom<usize>`] impl below, which is the only place the bound is
/// checked. [`Fd::from_number_unchecked`] bypasses it for callers that already hold the proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fd(u32);

impl Fd {
    /// Names descriptor `number` without checking the bound.
    ///
    /// The caller must ensure `number` is below [`MAX_FD`]. This constructor performs no
    /// validation; an out-of-range descriptor is reported as not open by every operation that takes
    /// one.
    pub(crate) const fn from_number_unchecked(number: usize) -> Self {
        Self(number as u32)
    }

    /// Returns the descriptor number.
    pub const fn number(self) -> usize {
        self.0 as usize
    }
}

impl TryFrom<usize> for Fd {
    type Error = InvalidFd;

    fn try_from(number: usize) -> Result<Self, Self::Error> {
        if number >= MAX_FD {
            return Err(InvalidFd(number));
        }
        Ok(Self(number as u32))
    }
}

/// Errors returned when converting a descriptor number into an [`Fd`].
///
/// The number is outside the table, so it names no descriptor at all. Nothing was looked up.
#[derive(Debug, thiserror::Error)]
#[error("Descriptor {0} is outside the table")]
pub struct InvalidFd(usize);
