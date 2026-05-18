//! Heap-free reservation bitmap for the virtual-memory manager.
//!
//! The `virtmem` module tracks outstanding virtual-address reservations so that
//! the random address search never hands out a range overlapping one. This
//! module provides that bookkeeping as a **one-bit-per-page bitmap** whose
//! backing storage lives in `virtmem`-owned `static` memory — no heap, no
//! allocator, no SVC calls.
//!
//! One bit per page is correct only because reservations are non-overlapping
//! page ranges: each page then belongs to at most one reservation, so
//! set-on-reserve / clear-on-release never collide. The non-overlap invariant
//! is validated at the edge by the caller, not here.
//!
//! [`ReservationMap`] is the stable interface. [`FlatReservationMap`] is the
//! flat (address-space-sized) implementation; [`RadixReservationMap`] is the
//! sparse two-level (guarded-radix) implementation whose resident footprint
//! tracks the *reserved area* rather than the address-space size. Both satisfy
//! the same interface, so `virtmem` callers swap between them without change.
//!
//! The guard-compression design behind [`RadixReservationMap`] — why the
//! directory is a sorted populated-chunk table and how that realises the
//! guarded-radix "guard" — is documented at the crate root.

use core::cell::UnsafeCell;

/// Log2 of the page size — every reservation is page-granular.
const PAGE_SHIFT: usize = 12;

/// Low-order bits that must be zero for a page-aligned address or size.
const PAGE_MASK: usize = (1 << PAGE_SHIFT) - 1;

/// Number of bits in one bitmap backing word.
const WORD_BITS: usize = usize::BITS as usize;

/// Size, in bytes, of the largest virtual-address region the `virtmem` manages.
///
/// The radix directory is sized to this worst case so a single `static`
/// serves every supported kernel. The widest region is the **36-bit-kernel
/// ASLR region** — `0x10_0000_0000` bytes (64 GiB) — the bound `init_state`
/// hardcodes on the legacy 36-bit detection path; the modern-kernel
/// `get_aslr_region_info` query never reports a wider one. Sizing to the
/// address-space span — not to a live-reservation count — is what keeps
/// reservation tracking structurally uncapped.
pub const MANAGED_SPAN: usize = 0x10_0000_0000;

/// Number of pages [`MANAGED_SPAN`] spans — one bitmap bit each.
///
/// `MANAGED_SPAN / PAGE_SIZE`; 16,777,216 pages for the 64 GiB worst case.
pub const MANAGED_PAGES: usize = MANAGED_SPAN >> PAGE_SHIFT;

/// A page-aligned virtual-address reservation.
///
/// Constructed only through [`Reservation::new`], which rejects unaligned or
/// empty ranges, so every value is a non-empty, page-aligned `[start, end)`
/// span. This is a self-describing value handle: it carries the exact extent,
/// which the bitmap alone cannot recover (abutting reservations would
/// clear-merge on release).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reservation {
    start: usize,
    size: usize,
}

impl Reservation {
    /// Creates a reservation, returning `None` unless `start` and `size` are
    /// both page-aligned, `size` is non-zero, and `start + size` does not
    /// overflow.
    pub fn new(start: usize, size: usize) -> Option<Self> {
        if size == 0 || (start & PAGE_MASK) != 0 || (size & PAGE_MASK) != 0 {
            return None;
        }
        // Reject ranges whose end would wrap the address space.
        start.checked_add(size)?;
        Some(Self { start, size })
    }

    /// Inclusive start address of the reserved range.
    pub const fn start(&self) -> usize {
        self.start
    }

    /// Byte length of the reserved range.
    pub const fn size(&self) -> usize {
        self.size
    }

    /// Exclusive end address of the reserved range.
    pub const fn end(&self) -> usize {
        self.start + self.size
    }
}

/// Stable interface for reservation bookkeeping.
///
/// [`FlatReservationMap`] and [`RadixReservationMap`] are interchangeable
/// implementations: swapping them is an internal change that does not touch the
/// `virtmem` callers.
pub trait ReservationMap {
    /// Marks every page in `range` as reserved.
    fn reserve(&mut self, range: Reservation);

    /// Clears the reservation for every page in `range`.
    fn release(&mut self, range: Reservation);

    /// Returns `true` if any page in `range` is currently reserved.
    fn is_reserved(&self, range: Reservation) -> bool;
}

/// Flat one-bit-per-page reservation bitmap.
///
/// Covers a contiguous `[base, base + pages * PAGE_SIZE)` span; bit `p` tracks
/// page `p` counted from `base`. The map owns no storage — it borrows a
/// caller-supplied `'static` word array. The footprint is fixed by the span size,
/// not by how many reservations are live.
///
/// Ranges passed to [`ReservationMap`] methods are intersected with the managed
/// span: a range falling partly or wholly outside is silently clamped (callers
/// reject out-of-span reservations at the edge, so this only ever trims the
/// guard padding of `is_reserved` queries).
pub struct FlatReservationMap {
    /// Base address mapped to bit 0.
    base: usize,
    /// Number of pages the bitmap covers.
    pages: usize,
    /// Backing words; `words.len() * WORD_BITS >= pages`.
    words: &'static mut [usize],
}

impl FlatReservationMap {
    /// Creates a flat bitmap over `pages` pages starting at `base`, backed by
    /// `words`.
    ///
    /// `words` must hold at least `pages` bits (`words.len() * WORD_BITS >=
    /// pages`); the backing is otherwise treated as exactly `pages` pages wide.
    pub fn new(base: usize, pages: usize, words: &'static mut [usize]) -> Self {
        debug_assert!(
            words.len() * WORD_BITS >= pages,
            "bitmap backing too small for the managed page count",
        );
        Self { base, pages, words }
    }

    /// Returns `true` if every page of `range` lies within the managed span.
    ///
    /// Callers validate this at the edge before [`reserve`](Self::reserve):
    /// an out-of-span range would otherwise be silently clamped, recording a
    /// reservation narrower than the caller asked for.
    pub fn contains(&self, range: Reservation) -> bool {
        span_contains(self.base, self.pages, range)
    }
}

impl ReservationMap for FlatReservationMap {
    fn reserve(&mut self, range: Reservation) {
        let Some((lo, hi)) = page_range(self.base, self.pages, range) else {
            return;
        };
        let words = &mut *self.words;
        for_each_word_mask(lo, hi, |word, mask| words[word] |= mask);
    }

    fn release(&mut self, range: Reservation) {
        let Some((lo, hi)) = page_range(self.base, self.pages, range) else {
            return;
        };
        let words = &mut *self.words;
        for_each_word_mask(lo, hi, |word, mask| words[word] &= !mask);
    }

    fn is_reserved(&self, range: Reservation) -> bool {
        let Some((lo, hi)) = page_range(self.base, self.pages, range) else {
            return false;
        };
        let mut reserved = false;
        for_each_word_mask(lo, hi, |word, mask| {
            reserved |= self.words[word] & mask != 0;
        });
        reserved
    }
}

/// Exclusive end address of a `[base, base + pages * PAGE_SIZE)` managed span.
///
/// Saturates rather than overflowing, so a span reaching the top of the address
/// space still yields a usable bound.
fn span_end(base: usize, pages: usize) -> usize {
    base.saturating_add(pages << PAGE_SHIFT)
}

/// Returns `true` if every page of `range` lies within `[base, span_end)`.
///
/// Shared by the flat and radix maps: out-of-span ranges must be rejected at
/// the edge, since the maps would otherwise silently clamp them.
fn span_contains(base: usize, pages: usize, range: Reservation) -> bool {
    range.start() >= base && range.end() <= span_end(base, pages)
}

/// Intersects `range` with the `[base, span_end)` managed span and converts it
/// to a `[lo, hi)` page-index range relative to `base`. Returns `None` when the
/// intersection is empty.
fn page_range(base: usize, pages: usize, range: Reservation) -> Option<(usize, usize)> {
    let end = span_end(base, pages);

    let lo_addr = range.start().max(base);
    let hi_addr = range.end().min(end);
    if lo_addr >= hi_addr {
        return None;
    }

    Some((
        (lo_addr - base) >> PAGE_SHIFT,
        (hi_addr - base) >> PAGE_SHIFT,
    ))
}

/// Invokes `f(word_index, mask)` once for each backing word overlapping the bit
/// range `[lo, hi)`, with `mask` selecting exactly the in-range bits of that
/// word. Partial first and last words are masked; fully covered words get an
/// all-ones mask.
fn for_each_word_mask(lo: usize, hi: usize, mut f: impl FnMut(usize, usize)) {
    let mut bit = lo;
    while bit < hi {
        let word = bit / WORD_BITS;
        let offset = bit % WORD_BITS;
        let word_end = (word + 1) * WORD_BITS;
        let chunk_end = hi.min(word_end);
        let count = chunk_end - bit;

        // `1 << WORD_BITS` would overflow, so the full-word case is explicit.
        let mask = if count == WORD_BITS {
            usize::MAX
        } else {
            ((1usize << count) - 1) << offset
        };

        f(word, mask);
        bit = chunk_end;
    }
}

/// Log2 of the number of pages one radix chunk covers.
///
/// A chunk spans [`RADIX_CHUNK_PAGES`] pages — `1 << 12` = 4096 pages, i.e.
/// 16 MiB of virtual address space. Where the flat bitmap dedicates a bit to
/// every page of the managed span up front, the radix map splits the span into
/// chunks and materialises a leaf bitmap for a chunk only once it holds a
/// reservation.
pub const RADIX_CHUNK_SHIFT: usize = 12;

/// Number of pages one radix chunk covers — `1 << RADIX_CHUNK_SHIFT`.
pub const RADIX_CHUNK_PAGES: usize = 1 << RADIX_CHUNK_SHIFT;

/// Number of chunks the managed span splits into — the chunk-index space.
///
/// `MANAGED_PAGES / RADIX_CHUNK_PAGES`; 4096 chunks for the 64 GiB worst case.
/// This is the *range* of valid chunk indices, not a storage size: the
/// guard-compressed directory holds an entry only for a populated chunk, so its
/// resident size tracks reserved area rather than this count.
pub const RADIX_CHUNK_COUNT: usize = MANAGED_PAGES >> RADIX_CHUNK_SHIFT;

/// Number of backing words in one leaf bitmap — `RADIX_CHUNK_PAGES / WORD_BITS`.
pub const RADIX_LEAF_WORDS: usize = RADIX_CHUNK_PAGES / WORD_BITS;

/// Number of leaf bitmaps in the pool.
///
/// A leaf is claimed for a chunk on its first reservation and returned to the
/// pool when the chunk empties, so the pool bounds the number of chunks holding
/// a reservation *concurrently* — not the address-space size. 256 leaves cover
/// 256 distinct 16 MiB chunks, generously above the handful of live `virtmem`
/// reservations (bounded in turn by the 128-slot FFI handle pool). It is a
/// whole multiple of [`WORD_BITS`] so the free bitmap carries no padding bits.
///
/// This also caps the guard-compressed directory: a populated chunk owns one
/// leaf *and* one directory entry, so the table can never hold more than
/// `RADIX_LEAF_COUNT` entries — leaf-pool exhaustion bounds it with no separate
/// overflow path.
pub const RADIX_LEAF_COUNT: usize = 256;

/// Number of words in the leaf-pool free bitmap.
const RADIX_FREE_WORDS: usize = RADIX_LEAF_COUNT / WORD_BITS;

const _: () = {
    assert!(
        RADIX_CHUNK_COUNT * RADIX_CHUNK_PAGES == MANAGED_PAGES,
        "the chunk grid must cover exactly the managed page count",
    );
    assert!(
        RADIX_CHUNK_PAGES.is_multiple_of(WORD_BITS),
        "a chunk must be a whole number of backing words",
    );
    assert!(
        RADIX_LEAF_COUNT.is_multiple_of(WORD_BITS),
        "the leaf count must fill the free bitmap with no padding bits",
    );
    assert!(
        RADIX_LEAF_COUNT <= RADIX_CHUNK_COUNT,
        "the directory cannot hold more entries than there are chunks",
    );
    assert!(
        RADIX_CHUNK_COUNT <= u16::MAX as usize,
        "a chunk index must fit the directory entry width",
    );
    assert!(
        RADIX_LEAF_COUNT <= u16::MAX as usize,
        "a leaf index must fit the directory entry width",
    );
};

/// Byte footprint of the [`RADIX`] `static` backing.
///
/// The guard-compressed directory (~1 KiB) plus the full leaf pool (~128 KiB)
/// — ~129 KiB for the tuning above, against the ~2 MiB an address-space-sized
/// flat bitmap would cost. Guard compression shrank the directory from the
/// ~16 KiB a flat one-slot-per-chunk array would take. This is the fixed `.bss`
/// cost; live occupancy is far lower, since only reserved chunks own a leaf and
/// a directory entry.
pub const RADIX_BYTES: usize = core::mem::size_of::<RadixBacking>();

/// `virtmem`-owned `static` backing for the two-level reservation map.
///
/// Holds the guard-compressed chunk directory, the leaf-bitmap pool, and the
/// pool's free bitmap. It is plain zero-initialised `.bss`: an empty directory
/// (`dir_len` of zero), a cleared leaf, and a free pool slot are all the
/// all-zero bit pattern, so no runtime initialisation is needed.
pub struct RadixBacking {
    /// Guard-compressed directory: a sorted sparse table. `dir[..dir_len]`
    /// holds one [`DirEntry`] per chunk that currently owns a leaf, ascending
    /// by chunk index; an unreserved chunk — and so any unreserved span between
    /// reservations — occupies no entry. Entries past `dir_len` are stale.
    dir: [DirEntry; RADIX_LEAF_COUNT],
    /// Number of live entries in `dir` — equivalently, the count of populated
    /// chunks and of claimed leaves (`popcount(used)`).
    dir_len: u16,
    /// Leaf-bitmap pool — each leaf is a one-bit-per-page bitmap for one chunk.
    leaves: [[usize; RADIX_LEAF_WORDS]; RADIX_LEAF_COUNT],
    /// Free bitmap: bit `i` set means leaf `i` is currently claimed.
    used: [usize; RADIX_FREE_WORDS],
}

impl RadixBacking {
    /// Creates empty backing — the directory is empty, no chunk owns a leaf,
    /// and the whole pool is free.
    const fn new() -> Self {
        Self {
            dir: [DirEntry { chunk: 0, leaf: 0 }; RADIX_LEAF_COUNT],
            dir_len: 0,
            leaves: [[0; RADIX_LEAF_WORDS]; RADIX_LEAF_COUNT],
            used: [0; RADIX_FREE_WORDS],
        }
    }
}

/// One entry of the guard-compressed directory: a populated chunk and the
/// leaf-pool slot serving it.
///
/// The directory is a sorted table of these. The "guard" of the guarded-radix
/// model is implicit and free: the gap between two adjacent entries' chunk
/// indices is exactly the run of unreserved chunks skipped between them, and it
/// costs no storage.
#[derive(Clone, Copy)]
struct DirEntry {
    /// Chunk index into the managed span — `0..RADIX_CHUNK_COUNT`.
    chunk: u16,
    /// Leaf-pool slot serving the chunk — indexes `RadixBacking::leaves`.
    leaf: u16,
}

/// `virtmem`-owned `static` storage for the sparse reservation map.
///
/// A [`RadixReservationMap`] borrows the inner [`RadixBacking`] as a
/// `&'static mut`. Acquiring it costs **no SVC call and no allocator call**.
///
/// It is mutated only through the map held by `VirtmemState`, reachable solely
/// under the `VIRTMEM` mutex, so the backing is never accessed concurrently.
pub static RADIX: RadixStorage = RadixStorage::new();

/// `Sync` wrapper that lets the interior-mutable radix backing live in a
/// `static`.
#[repr(transparent)]
pub struct RadixStorage(UnsafeCell<RadixBacking>);

// SAFETY: the backing is mutated only through the `RadixReservationMap` owned by
// `VirtmemState`, itself reachable solely under the `VIRTMEM` mutex, so no two
// threads ever touch the cell concurrently.
unsafe impl Sync for RadixStorage {}

impl RadixStorage {
    /// Creates empty, all-free backing.
    const fn new() -> Self {
        Self(UnsafeCell::new(RadixBacking::new()))
    }

    /// Returns a raw pointer to the backing.
    ///
    /// The caller turns this into the `&'static mut RadixBacking` a
    /// [`RadixReservationMap`] borrows; doing so soundly requires that no other
    /// reference to the backing is live — the `VIRTMEM` mutex provides that.
    pub fn get(&self) -> *mut RadixBacking {
        self.0.get()
    }
}

/// Sparse two-level (guarded-radix) reservation map.
///
/// Covers the same `[base, base + pages * PAGE_SIZE)` span as
/// [`FlatReservationMap`] and satisfies the same [`ReservationMap`] interface,
/// but splits the span into [`RADIX_CHUNK_PAGES`]-page chunks. A chunk's leaf
/// bitmap is claimed from the pool on its first reservation and returned when
/// the chunk empties, and the chunk is found through a guard-compressed
/// directory — a sorted table holding an entry only for a populated chunk — so
/// the resident footprint tracks the *reserved area* rather than the
/// address-space size. See the crate-root docs for the design rationale.
///
/// The map owns no storage — it borrows a `virtmem`-owned `static` [`RadixBacking`].
pub struct RadixReservationMap {
    /// Base address mapped to page 0.
    base: usize,
    /// Number of pages the map covers; never exceeds [`MANAGED_PAGES`].
    pages: usize,
    /// Borrowed `virtmem`-owned directory, leaf pool, and free bitmap.
    backing: &'static mut RadixBacking,
}

impl RadixReservationMap {
    /// Creates a radix map over `pages` pages starting at `base`, backed by
    /// `backing`.
    ///
    /// `pages` must not exceed [`MANAGED_PAGES`] — the directory holds exactly
    /// one entry per chunk of that worst-case span.
    pub fn new(base: usize, pages: usize, backing: &'static mut RadixBacking) -> Self {
        debug_assert!(
            pages <= MANAGED_PAGES,
            "managed page count exceeds the radix directory span",
        );
        Self {
            base,
            pages,
            backing,
        }
    }

    /// Returns `true` if every page of `range` lies within the managed span.
    ///
    /// The radix analogue of [`FlatReservationMap::contains`]; callers reject
    /// out-of-span reservations at the edge with it.
    pub fn contains(&self, range: Reservation) -> bool {
        span_contains(self.base, self.pages, range)
    }

    /// Number of leaves currently claimed from the pool.
    ///
    /// Equals the count of chunks holding at least one reservation — the
    /// observable footprint the radix map keeps proportional to reserved area
    /// rather than to address-space size.
    pub fn leaves_in_use(&self) -> usize {
        self.backing
            .used
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    /// Number of directory entries currently materialised.
    ///
    /// Equals the count of populated chunks. A guard-compressed directory
    /// spends an entry only on a populated chunk, so this is the directory's
    /// true resident size — far below [`RADIX_CHUNK_COUNT`] for the sparse,
    /// clustered reservation patterns `virtmem` produces.
    pub fn directory_len(&self) -> usize {
        self.backing.dir_len as usize
    }

    /// Returns the index in `dir` of the entry for `chunk`, or `None` when the
    /// chunk owns no leaf. Binary search over the sorted directory.
    fn find(&self, chunk: usize) -> Option<usize> {
        debug_assert!(chunk < RADIX_CHUNK_COUNT, "chunk index out of range");
        let len = self.backing.dir_len as usize;
        self.backing.dir[..len]
            .binary_search_by_key(&(chunk as u16), |entry| entry.chunk)
            .ok()
    }

    /// Returns the leaf index serving `chunk`, claiming a free pool leaf and
    /// inserting a directory entry on first use. Returns `None` only when the
    /// leaf pool is exhausted, in which case the directory is left unchanged.
    fn ensure_leaf(&mut self, chunk: usize) -> Option<usize> {
        debug_assert!(chunk < RADIX_CHUNK_COUNT, "chunk index out of range");
        let len = self.backing.dir_len as usize;
        let slot = match self.backing.dir[..len]
            .binary_search_by_key(&(chunk as u16), |entry| entry.chunk)
        {
            Ok(slot) => return Some(self.backing.dir[slot].leaf as usize),
            Err(slot) => slot,
        };
        let leaf = first_free_leaf(&self.backing.used)?;
        self.backing.used[leaf / WORD_BITS] |= 1usize << (leaf % WORD_BITS);
        // Open a gap at `slot` so the directory stays ascending by chunk.
        debug_assert!(len < RADIX_LEAF_COUNT, "directory table overflow");
        self.backing.dir.copy_within(slot..len, slot + 1);
        self.backing.dir[slot] = DirEntry {
            chunk: chunk as u16,
            leaf: leaf as u16,
        };
        self.backing.dir_len += 1;
        Some(leaf)
    }

    /// Drops the directory entry at `slot` and returns its leaf to the pool —
    /// the chunk it served is now empty.
    fn drop_chunk(&mut self, slot: usize) {
        let leaf = self.backing.dir[slot].leaf as usize;
        let len = self.backing.dir_len as usize;
        // Close the gap at `slot`, keeping the directory ascending by chunk.
        self.backing.dir.copy_within(slot + 1..len, slot);
        self.backing.dir_len -= 1;
        self.backing.used[leaf / WORD_BITS] &= !(1usize << (leaf % WORD_BITS));
    }
}

impl ReservationMap for RadixReservationMap {
    fn reserve(&mut self, range: Reservation) {
        let Some((lo, hi)) = page_range(self.base, self.pages, range) else {
            return;
        };
        for_each_chunk_segment(lo, hi, |chunk, local_lo, local_hi| {
            // Pool exhaustion leaves the segment untracked; RADIX_LEAF_COUNT is
            // sized so this cannot happen for the supported reservation load.
            let Some(leaf) = self.ensure_leaf(chunk) else {
                return;
            };
            let words = &mut self.backing.leaves[leaf];
            for_each_word_mask(local_lo, local_hi, |word, mask| words[word] |= mask);
        });
    }

    fn release(&mut self, range: Reservation) {
        let Some((lo, hi)) = page_range(self.base, self.pages, range) else {
            return;
        };
        for_each_chunk_segment(lo, hi, |chunk, local_lo, local_hi| {
            let Some(slot) = self.find(chunk) else {
                return;
            };
            let leaf = self.backing.dir[slot].leaf as usize;
            let words = &mut self.backing.leaves[leaf];
            for_each_word_mask(local_lo, local_hi, |word, mask| words[word] &= !mask);
            // Drop the chunk's directory entry and leaf once it holds nothing.
            if words.iter().all(|&word| word == 0) {
                self.drop_chunk(slot);
            }
        });
    }

    fn is_reserved(&self, range: Reservation) -> bool {
        let Some((lo, hi)) = page_range(self.base, self.pages, range) else {
            return false;
        };
        let mut reserved = false;
        for_each_chunk_segment(lo, hi, |chunk, local_lo, local_hi| {
            if reserved {
                return;
            }
            let Some(slot) = self.find(chunk) else {
                return;
            };
            let words = &self.backing.leaves[self.backing.dir[slot].leaf as usize];
            for_each_word_mask(local_lo, local_hi, |word, mask| {
                reserved |= words[word] & mask != 0;
            });
        });
        reserved
    }
}

/// Invokes `f(chunk, local_lo, local_hi)` once for each radix chunk overlapping
/// the page range `[lo, hi)`, where `[local_lo, local_hi)` is that chunk's
/// in-chunk page sub-range (both bounds relative to the chunk base).
fn for_each_chunk_segment(lo: usize, hi: usize, mut f: impl FnMut(usize, usize, usize)) {
    let mut page = lo;
    while page < hi {
        let chunk = page >> RADIX_CHUNK_SHIFT;
        let chunk_base = chunk << RADIX_CHUNK_SHIFT;
        let seg_end = hi.min(chunk_base + RADIX_CHUNK_PAGES);
        f(chunk, page - chunk_base, seg_end - chunk_base);
        page = seg_end;
    }
}

/// Returns the index of the first free leaf in the pool, or `None` when every
/// leaf is claimed. `used` bit `i` set means leaf `i` is in use; the pool has
/// no padding bits, so any clear bit is a valid leaf index.
fn first_free_leaf(used: &[usize]) -> Option<usize> {
    let word = used.iter().position(|&w| w != usize::MAX)?;
    Some(word * WORD_BITS + used[word].trailing_ones() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Page size in bytes — derived from [`PAGE_SHIFT`] for readable fixtures.
    const PAGE_SIZE: usize = 1 << PAGE_SHIFT;

    /// Base address used by the test fixtures (the 36-bit ASLR base).
    const BASE: usize = 0x8000000;

    /// Builds a flat map over `pages` pages with freshly zeroed backing.
    fn fresh_map(pages: usize) -> FlatReservationMap {
        let word_count = pages.div_ceil(WORD_BITS);
        let backing = alloc::vec![0usize; word_count].into_boxed_slice();
        FlatReservationMap::new(BASE, pages, alloc::boxed::Box::leak(backing))
    }

    /// Builds a reservation spanning `[BASE + start_page, +page_count)` pages.
    fn rsv(start_page: usize, page_count: usize) -> Reservation {
        Reservation::new(BASE + start_page * PAGE_SIZE, page_count * PAGE_SIZE)
            .expect("test fixture should be page-aligned and non-empty")
    }

    #[test]
    fn new_with_aligned_args_returns_some() {
        //* Given
        let start = BASE;
        let size = 4 * PAGE_SIZE;

        //* When
        let result = Reservation::new(start, size);

        //* Then
        assert!(
            result.is_some(),
            "aligned, non-empty range should construct"
        );
        let reservation = result.expect("should return reservation");
        assert_eq!(reservation.start(), start, "start should round-trip");
        assert_eq!(
            reservation.end(),
            start + size,
            "end should be start + size"
        );
    }

    #[test]
    fn new_with_unaligned_start_returns_none() {
        //* Given
        let unaligned_start = BASE + 1;

        //* When
        let result = Reservation::new(unaligned_start, PAGE_SIZE);

        //* Then
        assert!(result.is_none(), "unaligned start should be rejected");
    }

    #[test]
    fn new_with_unaligned_size_returns_none() {
        //* Given
        let unaligned_size = PAGE_SIZE + 1;

        //* When
        let result = Reservation::new(BASE, unaligned_size);

        //* Then
        assert!(result.is_none(), "unaligned size should be rejected");
    }

    #[test]
    fn new_with_zero_size_returns_none() {
        //* When
        let result = Reservation::new(BASE, 0);

        //* Then
        assert!(result.is_none(), "zero-size reservation should be rejected");
    }

    #[test]
    fn is_reserved_with_empty_map_returns_false() {
        //* Given
        let map = fresh_map(256);

        //* When
        let reserved = map.is_reserved(rsv(0, 256));

        //* Then
        assert!(!reserved, "no page is reserved in a fresh map");
    }

    #[test]
    fn is_reserved_with_reserved_range_returns_true() {
        //* Given
        let mut map = fresh_map(256);
        map.reserve(rsv(10, 4));

        //* When
        let reserved = map.is_reserved(rsv(10, 4));

        //* Then
        assert!(reserved, "the reserved range should read back as reserved");
    }

    #[test]
    fn is_reserved_with_disjoint_range_returns_false() {
        //* Given
        let mut map = fresh_map(256);
        map.reserve(rsv(10, 4));

        //* When
        let reserved = map.is_reserved(rsv(20, 4));

        //* Then
        assert!(!reserved, "a range not touching the reservation is free");
    }

    #[test]
    fn is_reserved_with_adjacent_range_returns_false() {
        //* Given
        let mut map = fresh_map(256);
        map.reserve(rsv(10, 4)); // pages 10..14

        //* When
        let reserved = map.is_reserved(rsv(14, 4)); // pages 14..18, abutting

        //* Then
        assert!(!reserved, "an abutting but non-overlapping range is free");
    }

    #[test]
    fn is_reserved_with_partial_overlap_returns_true() {
        //* Given
        let mut map = fresh_map(256);
        map.reserve(rsv(10, 4)); // pages 10..14

        //* When
        let reserved = map.is_reserved(rsv(13, 4)); // pages 13..17, overlaps 13

        //* Then
        assert!(reserved, "a range sharing one page is reserved");
    }

    #[test]
    fn is_reserved_after_release_returns_false() {
        //* Given
        let mut map = fresh_map(256);
        let range = rsv(10, 4);
        map.reserve(range);
        map.release(range);

        //* When
        let reserved = map.is_reserved(range);

        //* Then
        assert!(!reserved, "a released range should read back as free");
    }

    #[test]
    fn release_with_adjacent_reservation_leaves_neighbour_set() {
        //* Given
        let mut map = fresh_map(256);
        map.reserve(rsv(10, 4)); // pages 10..14
        map.reserve(rsv(14, 4)); // pages 14..18

        //* When
        map.release(rsv(10, 4));

        //* Then
        assert!(
            !map.is_reserved(rsv(10, 4)),
            "the released range should be free",
        );
        assert!(
            map.is_reserved(rsv(14, 4)),
            "the abutting reservation should be untouched",
        );
    }

    #[test]
    fn is_reserved_with_cross_word_range_returns_true() {
        //* Given
        let mut map = fresh_map(4 * WORD_BITS);
        // Span a word boundary: last page of word 0 through first of word 1.
        map.reserve(rsv(WORD_BITS - 1, 2));

        //* When
        let reserved = map.is_reserved(rsv(WORD_BITS, 1));

        //* Then
        assert!(reserved, "the page in the next word should be reserved");
    }

    #[test]
    fn is_reserved_with_cross_word_neighbour_returns_false() {
        //* Given
        let mut map = fresh_map(4 * WORD_BITS);
        map.reserve(rsv(WORD_BITS - 1, 2)); // pages WB-1 .. WB+1

        //* When
        let reserved = map.is_reserved(rsv(WORD_BITS + 1, 1));

        //* Then
        assert!(
            !reserved,
            "the partial-word mask must not bleed past the range"
        );
    }

    #[test]
    fn is_reserved_with_out_of_span_range_returns_false() {
        //* Given
        let map = fresh_map(64);
        // A reservation entirely above the managed span.
        let beyond =
            Reservation::new(BASE + 4096 * PAGE_SIZE, PAGE_SIZE).expect("fixture should construct");

        //* When
        let reserved = map.is_reserved(beyond);

        //* Then
        assert!(!reserved, "an out-of-span query is clamped away to free");
    }

    #[test]
    fn reserve_with_out_of_span_range_is_noop() {
        //* Given
        let mut map = fresh_map(64);
        let beyond =
            Reservation::new(BASE + 4096 * PAGE_SIZE, PAGE_SIZE).expect("fixture should construct");

        //* When
        map.reserve(beyond);

        //* Then
        assert!(
            !map.is_reserved(rsv(0, 64)),
            "an out-of-span reserve must not set any in-span bit",
        );
    }

    #[test]
    fn is_reserved_with_full_span_reservation_returns_true() {
        //* Given
        let mut map = fresh_map(3 * WORD_BITS + 7);
        map.reserve(rsv(0, 3 * WORD_BITS + 7));

        //* When
        let reserved = map.is_reserved(rsv(3 * WORD_BITS + 6, 1));

        //* Then
        assert!(reserved, "the last page of a full-span reserve is set");
    }

    /// Unit tests for the sparse two-level [`RadixReservationMap`].
    mod radix {
        use super::{super::*, BASE, PAGE_SIZE, rsv};

        /// Builds a radix map over `pages` pages with freshly zeroed backing.
        fn fresh_radix(pages: usize) -> RadixReservationMap {
            let backing = alloc::boxed::Box::new(RadixBacking::new());
            RadixReservationMap::new(BASE, pages, alloc::boxed::Box::leak(backing))
        }

        #[test]
        fn reserve_with_in_chunk_range_reads_back_reserved() {
            //* Given
            let mut map = fresh_radix(RADIX_CHUNK_PAGES);

            //* When
            map.reserve(rsv(10, 4));

            //* Then
            assert!(
                map.is_reserved(rsv(10, 4)),
                "the reserved range should read back as reserved",
            );
        }

        #[test]
        fn is_reserved_with_empty_map_returns_false() {
            //* Given
            let map = fresh_radix(RADIX_CHUNK_PAGES);

            //* When
            let reserved = map.is_reserved(rsv(0, RADIX_CHUNK_PAGES));

            //* Then
            assert!(!reserved, "no chunk owns a leaf in a fresh map");
        }

        #[test]
        fn is_reserved_with_disjoint_range_returns_false() {
            //* Given
            let mut map = fresh_radix(RADIX_CHUNK_PAGES);
            map.reserve(rsv(10, 4));

            //* When
            let reserved = map.is_reserved(rsv(20, 4));

            //* Then
            assert!(!reserved, "a range not touching the reservation is free");
        }

        #[test]
        fn is_reserved_with_partial_overlap_returns_true() {
            //* Given
            let mut map = fresh_radix(RADIX_CHUNK_PAGES);
            map.reserve(rsv(10, 4)); // pages 10..14

            //* When
            let reserved = map.is_reserved(rsv(13, 4)); // pages 13..17

            //* Then
            assert!(reserved, "a range sharing one page is reserved");
        }

        #[test]
        fn is_reserved_after_release_returns_false() {
            //* Given
            let mut map = fresh_radix(RADIX_CHUNK_PAGES);
            let range = rsv(10, 4);
            map.reserve(range);
            map.release(range);

            //* When
            let reserved = map.is_reserved(range);

            //* Then
            assert!(!reserved, "a released range should read back as free");
        }

        #[test]
        fn reserve_with_chunk_spanning_range_marks_both_chunks() {
            //* Given
            let mut map = fresh_radix(2 * RADIX_CHUNK_PAGES);

            //* When
            // pages CHUNK-2 .. CHUNK+2 — straddling the chunk 0 / chunk 1 edge.
            map.reserve(rsv(RADIX_CHUNK_PAGES - 2, 4));

            //* Then
            assert!(
                map.is_reserved(rsv(RADIX_CHUNK_PAGES - 2, 1)),
                "the page in the first chunk should be reserved",
            );
            assert!(
                map.is_reserved(rsv(RADIX_CHUNK_PAGES, 1)),
                "the page in the second chunk should be reserved",
            );
            assert_eq!(
                map.leaves_in_use(),
                2,
                "a range crossing a chunk edge claims one leaf per chunk",
            );
        }

        #[test]
        fn reserve_with_far_apart_ranges_claims_one_leaf_per_chunk() {
            //* Given
            let mut map = fresh_radix(MANAGED_PAGES);
            map.reserve(rsv(0, 1));

            //* When
            // 100 chunks higher — far apart, yet only two leaves are claimed.
            map.reserve(rsv(100 * RADIX_CHUNK_PAGES, 1));

            //* Then
            assert_eq!(
                map.leaves_in_use(),
                2,
                "leaf occupancy tracks distinct chunks touched, not address span",
            );
            assert!(
                map.is_reserved(rsv(0, 1)),
                "the low reservation should still read back as reserved",
            );
            assert!(
                map.is_reserved(rsv(100 * RADIX_CHUNK_PAGES, 1)),
                "the high reservation should read back as reserved",
            );
        }

        #[test]
        fn release_when_chunk_empties_returns_leaf_to_pool() {
            //* Given
            let mut map = fresh_radix(RADIX_CHUNK_PAGES);
            let range = rsv(10, 4);
            map.reserve(range);
            assert_eq!(map.leaves_in_use(), 1, "the reserve should claim a leaf");

            //* When
            map.release(range);

            //* Then
            assert_eq!(
                map.leaves_in_use(),
                0,
                "emptying a chunk returns its leaf to the pool",
            );
        }

        #[test]
        fn release_with_remaining_reservation_keeps_leaf() {
            //* Given
            let mut map = fresh_radix(RADIX_CHUNK_PAGES);
            map.reserve(rsv(10, 4));
            map.reserve(rsv(20, 4)); // same chunk

            //* When
            map.release(rsv(10, 4));

            //* Then
            assert_eq!(
                map.leaves_in_use(),
                1,
                "the chunk still holds a reservation, so its leaf is kept",
            );
            assert!(
                map.is_reserved(rsv(20, 4)),
                "the surviving reservation should be untouched",
            );
            assert!(
                !map.is_reserved(rsv(10, 4)),
                "the released range should be free",
            );
        }

        #[test]
        fn reserve_with_out_of_span_range_is_noop() {
            //* Given
            let mut map = fresh_radix(64);
            let beyond = Reservation::new(BASE + 4096 * PAGE_SIZE, PAGE_SIZE)
                .expect("fixture should construct");

            //* When
            map.reserve(beyond);

            //* Then
            assert_eq!(
                map.leaves_in_use(),
                0,
                "an out-of-span reserve must not claim any leaf",
            );
            assert!(
                !map.is_reserved(rsv(0, 64)),
                "an out-of-span reserve must not set any in-span bit",
            );
        }

        #[test]
        fn directory_len_with_empty_map_returns_zero() {
            //* Given
            let map = fresh_radix(RADIX_CHUNK_PAGES);

            //* When
            let len = map.directory_len();

            //* Then
            assert_eq!(len, 0, "a fresh map materialises no directory entries");
        }

        #[test]
        fn reserve_with_far_apart_ranges_keeps_directory_sparse() {
            //* Given
            let mut map = fresh_radix(MANAGED_PAGES);
            map.reserve(rsv(0, 1));

            //* When
            // 100 chunks higher — the 99-chunk gap between costs no entry.
            map.reserve(rsv(100 * RADIX_CHUNK_PAGES, 1));

            //* Then
            assert_eq!(
                map.directory_len(),
                2,
                "the directory holds an entry only for the two populated chunks",
            );
        }

        #[test]
        fn release_when_chunk_empties_drops_directory_entry() {
            //* Given
            let mut map = fresh_radix(RADIX_CHUNK_PAGES);
            let range = rsv(10, 4);
            map.reserve(range);
            assert_eq!(map.directory_len(), 1, "the reserve should add an entry");

            //* When
            map.release(range);

            //* Then
            assert_eq!(
                map.directory_len(),
                0,
                "emptying a chunk drops its directory entry",
            );
        }

        #[test]
        fn release_with_middle_chunk_keeps_other_entries_queryable() {
            //* Given
            let mut map = fresh_radix(9 * RADIX_CHUNK_PAGES);
            // Reserve out of chunk order so the sorted insert shifts entries.
            map.reserve(rsv(5 * RADIX_CHUNK_PAGES, 1));
            map.reserve(rsv(2 * RADIX_CHUNK_PAGES, 1));
            map.reserve(rsv(8 * RADIX_CHUNK_PAGES, 1));

            //* When
            map.release(rsv(5 * RADIX_CHUNK_PAGES, 1));

            //* Then
            assert_eq!(
                map.directory_len(),
                2,
                "releasing the middle chunk drops exactly one entry",
            );
            assert!(
                map.is_reserved(rsv(2 * RADIX_CHUNK_PAGES, 1)),
                "the lower chunk should still read back as reserved",
            );
            assert!(
                map.is_reserved(rsv(8 * RADIX_CHUNK_PAGES, 1)),
                "the higher chunk should still read back as reserved",
            );
            assert!(
                !map.is_reserved(rsv(5 * RADIX_CHUNK_PAGES, 1)),
                "the released chunk should be free",
            );
        }
    }
}
