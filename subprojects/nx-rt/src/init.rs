//! Runtime initialization functions.

use crate::env::heap_override;

/// Initialize the allocator heap.
///
/// Uses heap override from loader config if available, otherwise allocates via SVC.
pub fn setup_heap() {
    match heap_override() {
        Some((addr, size)) => {
            // SAFETY: The loader guarantees this region is valid and owned by us.
            unsafe { nx_alloc::global::init_with_heap_override(addr, size) };
        }
        None => {
            nx_alloc::global::init();
        }
    }
}
