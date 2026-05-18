//! Memory alignment utilities for page-aligned operations.

/// Page size constant (4 KiB).
pub const PAGE_SIZE: usize = 0x1000;

/// Page mask for alignment operations.
const PAGE_MASK: usize = PAGE_SIZE - 1;

/// Checks if a size is page-aligned.
///
/// A size is considered page-aligned if it's a multiple of [`PAGE_SIZE`] (0x1000).
#[inline]
pub const fn is_page_aligned(size: usize) -> bool {
    size & PAGE_MASK == 0
}

/// Rounds up a size to the next page boundary.
///
/// If the size is already page-aligned, it returns the same value.
#[inline]
pub const fn round_up_to_page(size: usize) -> usize {
    if size == 0 {
        0
    } else {
        (size + PAGE_MASK) & !PAGE_MASK
    }
}

/// Rounds down a size to the previous page boundary.
///
/// If the size is already page-aligned, it returns the same value.
#[inline]
pub const fn round_down_to_page(size: usize) -> usize {
    size & !PAGE_MASK
}

/// Calculates the number of pages needed for a given size.
///
/// This rounds up to ensure all bytes are covered.
#[inline]
pub const fn pages_needed(size: usize) -> usize {
    if size == 0 {
        0
    } else {
        (size + PAGE_MASK) / PAGE_SIZE
    }
}
