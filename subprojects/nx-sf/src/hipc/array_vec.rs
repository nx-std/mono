//! Bounded, stack-allocated push-only vector backing the HIPC descriptor
//! accumulators in [`super::request`].
//!
//! HIPC descriptor counts are capped by 4-bit header fields, so the builder
//! never needs heap allocation — each kind owns an inline buffer plus a
//! length byte. [`ArrayVec`] wraps that pair so call sites push via a
//! method instead of reaching into raw tuple fields.

use core::mem::MaybeUninit;

/// Push-only inline vector with a compile-time capacity of `N` elements.
///
/// Slots are stored as [`MaybeUninit<T>`] so no default value or `Copy` bound
/// is required — only indices `0..len` are ever assumed initialized.
pub(crate) struct ArrayVec<T, const N: usize> {
    buf: [MaybeUninit<T>; N],
    len: u8,
}

impl<T, const N: usize> ArrayVec<T, N> {
    /// Constructs an empty vector. The backing slots are left uninitialized.
    #[inline]
    pub(crate) const fn new() -> Self {
        Self {
            buf: [const { MaybeUninit::uninit() }; N],
            len: 0,
        }
    }

    /// Appends `value`. In debug builds, panics if the capacity is exceeded;
    /// the wire-format cap is hardware-fixed so this should never trip in
    /// release.
    #[inline]
    pub(crate) fn push(&mut self, value: T) {
        let idx = self.len as usize;
        debug_assert!(idx < N, "ArrayVec capacity exceeded ({N})");
        self.buf[idx].write(value);
        self.len += 1;
    }

    /// Number of pushed elements.
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.len as usize
    }

    /// Borrow of the initialized prefix `[0..len]`.
    #[inline]
    pub(crate) fn as_slice(&self) -> &[T] {
        let init = &self.buf[..self.len as usize];
        // SAFETY: `push` writes the slot before bumping `len`,
        // so every index in `0..len` is initialized.
        unsafe { init.assume_init_ref() }
    }
}

impl<T, const N: usize> Default for ArrayVec<T, N> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone, const N: usize> Clone for ArrayVec<T, N> {
    fn clone(&self) -> Self {
        let mut out = Self::new();
        for value in self.as_slice() {
            out.push(value.clone());
        }
        out
    }
}

impl<T: core::fmt::Debug, const N: usize> core::fmt::Debug for ArrayVec<T, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}

impl<T, const N: usize> core::ops::Deref for ArrayVec<T, N> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, const N: usize> Drop for ArrayVec<T, N> {
    fn drop(&mut self) {
        let init = &mut self.buf[..self.len as usize];
        // SAFETY: same invariant as `as_slice` — indices `0..len` were
        // initialized by `push` and have not been read out.
        unsafe { init.assume_init_drop() };
    }
}
