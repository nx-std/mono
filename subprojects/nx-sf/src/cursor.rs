//! Zerocopy parsing cursor and the [`ResponsePayload`] composition trait.
//!
//! Response parsing across HIPC, CMIF, and TIPC walks the same shape:
//! align the slice, read one or more zerocopy headers, optionally read
//! variable-length tails. [`Cursor`] is the four-primitive walker that
//! threads the remaining bytes between reads and returns each section
//! via [`zerocopy::FromBytes::ref_from_prefix`] (and friends), no
//! allocations and no copies.
//!
//! [`ResponsePayload`] is the composition axis on top: every protocol
//! `parse_response` function is generic over `P: ResponsePayload`, and
//! call sites pick the payload shape via turbofish — `&MyHeader` for a
//! zerocopy struct, or `()` when the response carries no in-band
//! payload. Runtime-sized byte payloads sit outside the trait: CMIF
//! exposes them via dedicated `parse_response_bytes` entry points so
//! sized callers never thread a length they don't use.
//!
//! The trait is sealed; only the two impls in this module exist.
//! Adding a third payload shape is a deliberate, in-crate change, not
//! something downstream code can hook in behind the protocol parsers.

use zerocopy::FromBytes as _;

/// Walks a byte slice section-by-section using zerocopy reads.
///
/// Each method threads `self` by value so chains are linear and the
/// borrow checker enforces single-use of the remaining slice. A read
/// that would run past the remaining bytes returns `None`; protocol
/// parsers translate the absence into their own typed error variant
/// so per-section context is preserved.
///
/// The type is `pub` so it can appear in the sealed
/// [`ResponsePayload::read`] signature, but the constructor and reader
/// methods are crate-internal — only the in-crate blanket impls can
/// actually walk a cursor.
pub struct Cursor<'a> {
    rest: &'a [u8],
}

impl<'a> Cursor<'a> {
    /// Wraps `buf` as a cursor positioned at byte 0.
    #[inline]
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        Self { rest: buf }
    }

    /// Returns the bytes the cursor has not yet consumed.
    #[inline]
    pub(crate) fn remaining(&self) -> &'a [u8] {
        self.rest
    }

    /// Skips leading bytes so the next read starts on an `align`-byte
    /// boundary measured from the cursor's current pointer.
    ///
    /// Saturates at the end of the slice if the required padding would
    /// exceed the remaining bytes — the next read will then return
    /// `None` rather than panic on an out-of-range split.
    #[inline]
    pub(crate) fn align_to(mut self, align: usize) -> Self {
        let pad = self.rest.as_ptr().align_offset(align).min(self.rest.len());
        let (_pad, rest) = self.rest.split_at(pad);
        self.rest = rest;
        self
    }

    /// Reads a single `T` off the prefix and advances the cursor.
    ///
    /// Returns `None` if the remaining bytes do not fit a `T`.
    #[inline]
    pub(crate) fn read<T>(self) -> Option<(&'a T, Self)>
    where
        T: zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
    {
        let (val, rest) = T::ref_from_prefix(self.rest).ok()?;
        Some((val, Self { rest }))
    }

    /// Reads a slice of `n` consecutive `T` off the prefix and advances
    /// the cursor.
    ///
    /// Returns `None` if the remaining bytes do not fit `n` elements.
    #[inline]
    pub(crate) fn read_slice<T>(self, n: usize) -> Option<(&'a [T], Self)>
    where
        T: zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
    {
        let (val, rest) = <[T]>::ref_from_prefix_with_elems(self.rest, n).ok()?;
        Some((val, Self { rest }))
    }

    /// Splits off `n` raw bytes from the prefix and advances the cursor.
    ///
    /// Returns `None` if fewer than `n` bytes remain.
    #[inline]
    pub(crate) fn read_bytes(self, n: usize) -> Option<(&'a [u8], Self)> {
        let (val, rest) = self.rest.split_at_checked(n)?;
        Some((val, Self { rest }))
    }
}

/// Payload decoder for a fixed-shape IPC response. The composition
/// axis: every protocol `parse_response` function is generic over a
/// `P: ResponsePayload`, and callers pick the payload shape via
/// turbofish.
///
/// Two sealed implementations cover the in-crate call sites:
///
/// - `()` — no in-band payload (e.g. control requests that only return
///   handles).
/// - `&'a T` — any zerocopy struct, parsed with
///   [`zerocopy::FromBytes::ref_from_prefix`].
///
/// Runtime-sized byte payloads (CMIF `OutRawData`) are handled outside
/// this trait by dedicated `parse_response_bytes` entry points that
/// take an explicit `payload_len`. Keeping bytes-mode off the trait
/// lets sized callers omit the length argument entirely.
///
/// The trait is sealed because the payload shape is a protocol-layer
/// design choice, not a hook for downstream code.
pub trait ResponsePayload<'a>: _priv::Sealed + Sized {
    /// Parses the payload off the cursor and returns the cursor for any
    /// trailing sections the caller still needs to read.
    ///
    /// Returns `None` if the remaining bytes do not fit the payload.
    fn read(cursor: Cursor<'a>) -> Option<(Self, Cursor<'a>)>;
}

impl<'a> ResponsePayload<'a> for () {
    #[inline]
    fn read(cursor: Cursor<'a>) -> Option<((), Cursor<'a>)> {
        Some(((), cursor))
    }
}

impl<'a, T> ResponsePayload<'a> for &'a T
where
    T: zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    #[inline]
    fn read(cursor: Cursor<'a>) -> Option<(&'a T, Cursor<'a>)> {
        cursor.read::<T>()
    }
}

mod _priv {
    pub trait Sealed {}

    impl Sealed for () {}
    impl<T> Sealed for &T where T: zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout {}
}
