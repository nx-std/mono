//! # nx-sys-virtmem
//!
//! Heap-free virtual-memory-manager page substrate for Horizon OS.
//!
//! This crate is the standalone home of the `virtmem` page substrate extracted
//! from `nx-sys-mem` (SPEC Task 5.3). Its dependency graph carries no
//! `nx-alloc` and no `#[global_allocator]`-requiring crate, so the
//! page-below-heap layering is compiler-enforced (IC-8): the reservation
//! bookkeeping cannot regress into consuming the heap it sits beneath.
//!
//! ## Reservation tracking
//!
//! `virtmem` hands out virtual-address ranges by picking a random candidate
//! and probing it; the probe must never collide with a range already handed
//! out. The [`reservation`](virtmem::reservation) module keeps that record — a
//! **one-bit-per-page** map of every outstanding reservation, so a candidate
//! range can be tested for overlap in a single pass. One bit per page is
//! correct only because reservations are non-overlapping page ranges: each
//! page then belongs to at most one reservation, so set-on-reserve /
//! clear-on-release never collide. That non-overlap invariant is validated at
//! the edge by the caller, not by the map.
//!
//! [`ReservationMap`](virtmem::reservation::ReservationMap) is the stable
//! interface; two implementations satisfy it.
//! [`FlatReservationMap`](virtmem::reservation::FlatReservationMap) dedicates
//! one bit to every page of the managed span up front — a fixed array sized by
//! *address-space size*, not by how many reservations are live (~2 MiB of
//! `.bss` for the 64 GiB worst case).
//! [`RadixReservationMap`](virtmem::reservation::RadixReservationMap) is the
//! sparse implementation this crate ships: its resident footprint tracks the
//! *reserved area* instead.
//!
//! ## Guard compression — the sparse reservation map
//!
//! The reserved set is tiny and clustered. `virtmem` holds only a handful of
//! live reservations at once (bounded by the 128-slot FFI handle pool), each a
//! small contiguous range, scattered across a 64 GiB address space. Spending
//! storage proportional to the *span* rather than the *occupancy* is almost
//! all waste.
//!
//! [`RadixReservationMap`](virtmem::reservation::RadixReservationMap) removes
//! that waste in two stages, both modelled on seL4's CSpace — a guarded radix:
//!
//! 1. **On-demand leaves.** The span is split into
//!    [`RADIX_CHUNK_PAGES`](virtmem::reservation::RADIX_CHUNK_PAGES)-page
//!    (16 MiB) chunks. A chunk's one-bit-per-page *leaf* bitmap is claimed from
//!    a fixed pool only on the chunk's first reservation and returned when the
//!    chunk empties. An unreserved chunk owns no leaf, so the pool's occupancy
//!    tracks the number of chunks *touched*, not the address-space size.
//!
//! 2. **Guard-compressed directory.** A leaf still has to be *found* from a
//!    chunk index. A flat directory — one slot per chunk of the worst-case
//!    span — would be a fixed ~16 KiB `.bss` array, resident in full even when
//!    a single chunk is reserved, and most of it would describe long
//!    unreserved spans between distant reservations. Guard compression skips
//!    those spans: the directory stores an entry only for a *populated* chunk.
//!
//! ### The guard, as a sorted table
//!
//! seL4's guarded page table attaches a *guard* — a skipped run of index bits
//! — to a directory node, so a stretch of the radix that never branches costs
//! no node. This crate realises the same idea without a node tree: the
//! directory is a **sorted table of populated chunks**, `(chunk, leaf)` pairs
//! kept ascending by chunk index. The guard is implicit and exact — the gap
//! between two adjacent entries' chunk indices *is* the skipped run, and it
//! occupies no storage. An empty map is a zero-length table; two reservations
//! 100 chunks apart are two adjacent entries with a 99-chunk guard between
//! them and nothing materialised in between.
//!
//! Lookup is a binary search over the populated entries (`O(log n)`, with `n`
//! at most [`RADIX_LEAF_COUNT`](virtmem::reservation::RADIX_LEAF_COUNT));
//! insert and remove shift the tail of the table to keep it sorted (`O(n)`, a
//! move of at most ~1 KiB). Both costs land only on `reserve` / `release`,
//! which run under the `virtmem` mutex at human-paced frequency — never on a
//! hot path. In exchange the directory's resident size drops from a fixed
//! ~16 KiB to four bytes per populated chunk: ~1 KiB of capacity, a few
//! entries in practice. The whole `RADIX` backing falls from ~144 KiB to
//! ~129 KiB, and — like every structure in this crate — it stays plain
//! zero-initialised `.bss`: an empty table is the all-zero bit pattern, so
//! there is no runtime initialisation and no SVC or allocator call (IC-4).
//!
//! ### Why a table, not a node tree
//!
//! A literal guarded-radix node tree would match seL4's wording more closely,
//! but for this crate it is the wrong trade. The directory it would replace is
//! only ~16 KiB; a node tree needs its own on-demand node pool, whose fixed
//! cost can easily *exceed* the array it replaces unless the fan-out is
//! carefully tuned, and it carries the guarded-trie node-split case —
//! materialising an intermediate node when a reservation lands inside a
//! previously-skipped span — a well-known source of subtle bugs. The sorted
//! table delivers the same "structure only for populated chunks" guarantee
//! with a bounded, branch-free structure that is trivial to prove correct and
//! to keep heap-free.
//!
//! ### Invariants
//!
//! - The directory is **sorted ascending by chunk index**; the binary search
//!   and the shift-on-insert / shift-on-remove both depend on it.
//! - A populated chunk owns **exactly one** directory entry and **exactly
//!   one** leaf, so the table's live length, the count of claimed leaves, and
//!   the number of reserved chunks are all equal. Leaf-pool exhaustion
//!   therefore bounds the table — it can never overflow, and needs no separate
//!   overflow path.
//! - One bit per page is correct only under the reservation non-overlap
//!   invariant, which is validated at the edge, not by the map.
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

// Heap types are needed only by the `#[cfg(test)]` reservation-map fixtures;
// the crate's shipped code is pure `core`, so this never enters the link graph.
#[cfg(test)]
extern crate alloc;

pub mod alignment;
pub mod virtmem;
