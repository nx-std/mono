//! Dynamic Thread-Local Storage (TLS) slot management.
//!
//! Horizon allocates a small **user-TLS** area inside every thread's Thread-
//! Local Storage block. libnx exposes four C helpers around that region:
//! `threadTlsAlloc`, `threadTlsFree`, `threadTlsGet`, and `threadTlsSet`. This
//! Rust module re-implements the same functionality while preserving the exact
//! ABI expected by Horizon and by C code linking against libnx.
//!
//! ## Design highlights
//! 1. **Global usage bit-mask** `SLOTS_USED_MASK` tracks which of the
//!    [`NUM_TLS_SLOTS`] dynamic slots are allocated.  Atomic RMW operations
//!    allow concurrent allocation/free without a global lock.
//! 2. **Destructor table** `SLOTS_DESTRUCTORS` stores an optional destructor
//!    per slot, invoked from `thread_exit` for the terminating thread.
//! 3. **Thread list fan-out** Upon allocation we eagerly set the new cell to
//!    `NULL` for every live thread via the intrusive thread list; future
//!    threads start zero-initialised.
//! 4. **Memory ordering** Both `slot_alloc` and `slot_free` use
//!    `compare_exchange` with `AcqRel`/`Acquire`; this is sufficient to ensure
//!    happens-before between bit-mask changes and destructor writes while
//!    avoiding the cost of `SeqCst` fences.
//! 5. **Safety contract** All public functions are `unsafe` because Rust
//!    cannot enforce correct lifetime, aliasing, and concurrency semantics of
//!    raw TLS pointers.

use core::{
    ffi::c_void,
    marker::PhantomData,
    mem,
    ptr::{self, NonNull},
    slice,
    sync::atomic::{self, AtomicU64, Ordering},
};

use crate::{
    registry,
    tls_region::{self, USER_TLS_REGION_BEGIN, USER_TLS_REGION_END},
};

/// The number of slots in the TLS region.
///
/// The TLS region is divided into slots of size `core::mem::size_of::<*mut c_void>()`.
///
/// The number of slots is calculated as the difference between the end and the beginning
/// of the user-mode TLS region, divided by the size of the slot.
const NUM_SLOTS: usize = (USER_TLS_REGION_END - USER_TLS_REGION_BEGIN) / size_of::<*mut c_void>();

/// Bitmask with the lowest `NUM_TLS_SLOTS` bits set to 1.
///
/// This is used to mask out bits beyond the valid slot range so that `trailing_zeros`
/// never reports an index ≥ `NUM_TLS_SLOTS`.
const VALID_SLOT_MASK: u64 = if NUM_SLOTS < 64 {
    (1u64 << NUM_SLOTS) - 1
} else {
    u64::MAX
};

/// TLS dynamic slots used bitmask.
///
/// The bitmask is used to track which slots are in use.
static SLOTS_USED_MASK: AtomicU64 = AtomicU64::new(0);

/// TLS dynamic slots destructors.
///
/// The destructors are used to clean up the TLS dynamic slots when the thread exits.
static mut SLOTS_DESTRUCTORS: [Option<fn(*mut c_void)>; NUM_SLOTS] = [None; NUM_SLOTS];

/// Allocates a new TLS dynamic slot and returns its ID.
///
/// The destructor is used to clean up the TLS dynamic slot when the thread exits.
pub unsafe fn slot_alloc(destructor: Option<fn(*mut c_void)>) -> Option<usize> {
    let mut current_mask = SLOTS_USED_MASK.load(Ordering::Acquire);

    let mod_id = loop {
        // Mask out bits beyond the valid slot range so that `trailing_zeros`
        // never reports an index ≥ `NUM_TLS_SLOTS`.
        let free_mask = !current_mask & VALID_SLOT_MASK;
        let slot = free_mask.trailing_zeros() as usize;
        if slot >= NUM_SLOTS {
            // No free slots available
            return None;
        }

        // Try to claim the bit first – only publish the destructor once we
        // have exclusive ownership of the slot.
        let new_mask = current_mask | (1u64 << slot);
        match SLOTS_USED_MASK.compare_exchange(
            current_mask,
            new_mask,
            Ordering::AcqRel, // success: also acts as a release for subsequent writes
            Ordering::Acquire, // failure: refresh current_mask
        ) {
            Ok(_) => {
                // We successfully claimed the slot – now publish the destructor.
                unsafe { SLOTS_DESTRUCTORS[slot] = destructor };

                // Ensure the destructor write is visible to other threads that
                // observe the updated bitmask.
                atomic::fence(Ordering::Release);

                break slot;
            }
            Err(actual) => {
                // Another thread raced us; retry with the updated mask.
                current_mask = actual;
            }
        }
    };

    // SAFETY: index validated above; slice lives as long as the function call
    let _ = unsafe { curr_thread_slot_set(mod_id, ptr::null_mut()) };

    // Set the slot to null in all threads.
    // SAFETY: index validated above; `for_each` guarantees that `th` points to a valid `Thread`.
    unsafe {
        registry::for_each(|th| {
            let _ = th.slot_set(mod_id, ptr::null_mut());
        });
    }

    Some(mod_id)
}

/// Frees a previously-allocated dynamic TLS slot.
///
/// Mirrors libnx's `threadTlsFree`.
///
/// Behaviour:
/// 1. Clears the destructor pointer associated with `mod_id` so a
///    concurrently–running `thread_exit` will no longer invoke it.  This is
///    done *before* the slot is published as free.
/// 2. Atomically clears the corresponding bit in [`SLOTS_USED_MASK`].  The
///    CAS uses `AcqRel` ordering, establishing a *release* edge after the
///    destructor store.  Any other thread that later observes the bit as
///    cleared (with an `Acquire`/`SeqCst` load) is guaranteed to see the
///    updated destructor value.
///
/// # Safety
/// * The caller must ensure `mod_id < NUM_TLS_SLOTS`.
/// * The caller must guarantee that no other *safe* code continues to access
///   the slot once it has been freed. Raw code that still holds the slot ID
///   must treat it as unusable.
/// * This function is **not** re-entrant for the same `slot_id`; freeing a
///   slot twice is undefined behaviour.
#[inline]
pub unsafe fn slot_free(mod_id: usize) {
    #[cfg(debug_assertions)]
    {
        use nx_svc::debug::{BreakReason, break_event};
        if mod_id >= NUM_SLOTS {
            // TODO: Add a proper error message here.
            // panic!("TLS slot out of bounds: {}", slot_id);
            break_event(BreakReason::Assert, 0, 0);
        }
    }

    // Clear the destructor pointer first so that any thread which still
    // observes the slot as allocated will see `None` instead of invoking an
    // invalid destructor.
    unsafe { SLOTS_DESTRUCTORS[mod_id] = None };

    // Atomically clear the bit in the global usage mask.
    let mut current_mask = SLOTS_USED_MASK.load(Ordering::Acquire);
    loop {
        let new_mask = current_mask & !(1u64 << mod_id);
        match SLOTS_USED_MASK.compare_exchange(
            current_mask,
            new_mask,
            Ordering::AcqRel,  // success: releases the destructor store
            Ordering::Acquire, // failure: refresh current mask
        ) {
            Ok(_) => break,                       // Successfully cleared the bit.
            Err(actual) => current_mask = actual, // Retry with updated mask.
        }
    }
}

/// Reads the raw pointer stored in the dynamic TLS slot with the given `slot_id`.
///
/// Mirrors libnx's `threadTlsGet`.
///
/// # Safety
/// - The caller must ensure the slots slice is not aliased mutably elsewhere.
#[inline]
pub unsafe fn curr_thread_slot_get(mod_id: usize) -> Result<*mut c_void, SlotError> {
    if mod_id >= NUM_SLOTS {
        return Err(SlotError::OutOfBounds(mod_id));
    }

    let slots = unsafe { curr_thread_slots() };
    let slots_slice = slots.as_slice();

    // SAFETY: index validated above; slice lives as long as the function call.
    let value = unsafe { ptr::read_volatile(&slots_slice[mod_id]) };

    Ok(value)
}

/// Writes `value` into the dynamic TLS slot with the given `mod_id`.
///
/// Mirrors libnx's `threadTlsSet`.
///
/// # Safety
/// - The caller must ensure the slice is not aliased mutably elsewhere.
#[inline]
pub unsafe fn curr_thread_slot_set(mod_id: usize, value: *mut c_void) -> Result<(), SlotError> {
    if mod_id >= NUM_SLOTS {
        return Err(SlotError::OutOfBounds(mod_id));
    }

    let mut slots = unsafe { curr_thread_slots() };
    let slots_slice = slots.as_slice_mut();

    // SAFETY: index validated above; slice lives as long as the function call.
    unsafe { ptr::write_volatile(&mut slots_slice[mod_id], value) }

    Ok(())
}

/// Returns a wrapper around the dynamic TLS slot array for the **current thread**.
///
/// # Safety
/// * The caller must ensure the returned slice is not aliased mutably elsewhere.
pub unsafe fn curr_thread_slots() -> Slots {
    // SAFETY: The caller must ensure the returned slice is not aliased mutably elsewhere.
    unsafe { Slots::from_raw_parts(tls_region::slots_ptr(), NUM_SLOTS) }
}

/// Returns a wrapper around the dynamic TLS slot array for the **current thread**.
///
/// # Safety
/// * The caller must ensure the returned slice is not aliased mutably elsewhere.
pub unsafe fn curr_thread_used_slots() -> UsedSlots {
    // SAFETY: The caller must ensure the slots are not aliased mutably elsewhere.
    let slots = unsafe { curr_thread_slots() };

    // Get a copy of the used slots mask.
    let mask = SLOTS_USED_MASK.load(Ordering::Acquire) & VALID_SLOT_MASK;

    // SAFETY: The mask is a copy of the used slots mask.
    unsafe { UsedSlots::new(slots, mask) }
}

/// A wrapper around the dynamic TLS slot array.
///
/// # Safety
/// * The caller must ensure the returned slice is not aliased mutably elsewhere.
/// * The caller must ensure the slice is not aliased mutably elsewhere.
pub struct Slots {
    slots_ptr: NonNull<*mut c_void>,
    slots_len: usize,
}

impl Slots {
    /// Creates a new `SlotsRef` from a raw pointer.
    ///
    /// # Safety
    /// * The caller must ensure the returned slice is not aliased mutably elsewhere.
    pub unsafe fn from_raw_parts(addr: NonNull<*mut c_void>, len: usize) -> Self {
        Self {
            slots_ptr: addr,
            slots_len: len,
        }
    }

    /// Creates a new `SlotsRef` from a raw pointer.
    ///
    /// # Safety
    /// * The caller must ensure the returned slice is not aliased mutably elsewhere.
    pub unsafe fn from_ptr(addr: NonNull<*mut c_void>) -> Self {
        // SAFETY: The caller ensures the pointer is valid.
        unsafe { Self::from_raw_parts(addr, NUM_SLOTS) }
    }

    /// Get a const pointer to the slots array.
    #[inline(always)]
    pub fn as_ptr(&self) -> *const *mut c_void {
        self.slots_ptr.as_ptr()
    }

    /// Get a mutable pointer to the slots array.
    #[inline(always)]
    pub fn as_mut_ptr(&mut self) -> *mut *mut c_void {
        self.slots_ptr.as_ptr() as *mut *mut c_void
    }

    /// Returns a slice covering the dynamic TLS slot array for the **current thread**.
    #[inline(always)]
    pub fn as_slice(&self) -> &[*mut c_void] {
        // SAFETY: The caller must ensure the returned slice is not aliased mutably elsewhere.
        unsafe { slice::from_raw_parts(self.as_ptr(), self.slots_len) }
    }

    /// Returns a mutable slice covering the dynamic TLS slot array for the **current thread**.
    #[inline(always)]
    pub fn as_slice_mut(&mut self) -> &mut [*mut c_void] {
        // SAFETY: The caller must ensure the returned slice is not aliased mutably elsewhere.
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.slots_len) }
    }

    /// Given a slot index, returns a pointer to the slot.
    #[inline]
    pub fn get(&self, index: usize) -> Result<*mut c_void, SlotError> {
        if index >= self.slots_len {
            return Err(SlotError::OutOfBounds(index));
        }

        Ok(self.as_slice()[index])
    }

    /// Given a slot index, sets the slot to the given value.
    #[inline]
    pub fn set(&mut self, index: usize, value: *mut c_void) -> Result<(), SlotError> {
        if index >= self.slots_len {
            return Err(SlotError::OutOfBounds(index));
        }

        self.as_slice_mut()[index] = value;

        Ok(())
    }

    /// Run the destructors for the slots that are currently in use.
    ///
    /// - The destructors are run in the order of the slots that are currently in use.
    /// - The destructor is only run if the slot is non-null and has a destructor registered.
    /// - The slot is set to `NULL` before the destructor is run.
    pub fn run_destructors(&mut self) {
        // Get a snapshot of the global slots usage mask.
        let used_slots_mask = SLOTS_USED_MASK.load(Ordering::Acquire) & VALID_SLOT_MASK;

        // SAFETY: The mask is a snapshot of the global slots usage mask.
        let used_slots_iter = unsafe { UsedSlotsIterMut::new(self.as_mut_ptr(), used_slots_mask) };
        for (mod_id, slot) in used_slots_iter {
            if slot.is_null() {
                continue;
            }

            if let Some(value) = NonNull::new(mem::replace(slot, ptr::null_mut())) {
                if let Some(dtor) = unsafe { SLOTS_DESTRUCTORS[mod_id] } {
                    dtor(value.as_ptr());
                }
            }
        }
    }
}

/// A wrapper around the dynamic TLS slot array.
pub struct UsedSlots {
    mask: u64,
    slots: Slots,
}

impl UsedSlots {
    /// Creates a new `UsedSlots` from a `Slots` and a mask.
    ///
    /// # Safety
    /// * The caller must ensure the used slots mask is valid.
    pub unsafe fn new(slots: Slots, mask: u64) -> Self {
        Self {
            mask: mask & VALID_SLOT_MASK,
            slots,
        }
    }

    /// Returns an iterator over the used slots.
    pub fn iter(&self) -> UsedSlotsIter<'_> {
        // SAFETY: The caller must ensure the mask is valid.
        unsafe { UsedSlotsIter::new(self.slots.as_ptr(), self.mask) }
    }

    /// Returns a mutable iterator over the used slots.
    pub fn iter_mut(&mut self) -> UsedSlotsIterMut<'_> {
        // SAFETY: The caller must ensure the mask is valid.
        unsafe { UsedSlotsIterMut::new(self.slots.as_mut_ptr(), self.mask) }
    }
}

/// Immutable iterator over **used** dynamic TLS slots for the **current thread**.
///
/// The iterator yields pairs of `(slot_id, value)` where `slot_id` is the global slot
/// index (`0..NUM_TLS_SLOTS`) and `value` is the raw pointer stored in that slot for
/// the current thread. Only slots whose allocation bit is currently set in
/// [`SLOTS_USED_MASK`] are visited – unallocated slots are skipped.
///
/// The iterator performs a *snapshot* of the allocation bit-mask when it is
/// created; slots allocated/freed afterwards will *not* be reflected during
/// iteration (this matches libnx semantics in e.g. `threadExit`).
#[derive(Clone)]
pub struct UsedSlotsIter<'a> {
    mask: u64,
    slots_ptr: *const *mut c_void,
    _marker: PhantomData<&'a [*mut c_void]>,
}

impl<'a> UsedSlotsIter<'a> {
    /// Creates a new immutable iterator over the used slots.
    ///
    /// # Safety
    /// * The caller must ensure the mask is valid.
    #[inline]
    unsafe fn new(slots_ptr: *const *mut c_void, mask: u64) -> Self {
        Self {
            mask,
            slots_ptr,
            _marker: PhantomData,
        }
    }
}

impl<'a> Iterator for UsedSlotsIter<'a> {
    type Item = (usize, &'a *mut c_void);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.mask == 0 {
            return None;
        }
        let i = self.mask.trailing_zeros() as usize;

        // Clear the lowest set bit
        self.mask &= !(1u64 << i);

        // SAFETY: The index `i` is obtained from the mask, which is guaranteed
        // to have bits set only for valid slot indices. The `slots_ptr` points
        // to a valid memory block of `NUM_SLOTS` slots.
        let slot = unsafe { &*self.slots_ptr.add(i) };
        Some((i, slot))
    }
}

/// Mutable iterator over **used** dynamic TLS slots for the **current thread**.
///
/// Yields `(slot_id, &mut *mut c_void)` so that callers can directly modify the
/// per-thread slot value in-place. As with [`UsedSlotsIter`], the allocation
/// mask is snap-shotted at iterator creation time.
pub struct UsedSlotsIterMut<'a> {
    mask: u64,
    slots_ptr: *mut *mut c_void,
    _marker: PhantomData<&'a mut [*mut c_void]>,
}

impl<'a> UsedSlotsIterMut<'a> {
    /// Creates a new *mutable* iterator over the used slots.
    ///
    /// # Safety
    /// * The caller must ensure the mask is valid.
    #[inline]
    unsafe fn new(slots_ptr: *mut *mut c_void, mask: u64) -> Self {
        Self {
            mask,
            slots_ptr,
            _marker: PhantomData,
        }
    }
}

impl<'a> Iterator for UsedSlotsIterMut<'a> {
    type Item = (usize, &'a mut *mut c_void);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.mask == 0 {
            return None;
        }
        let i = self.mask.trailing_zeros() as usize;

        // Clear the lowest set bit
        self.mask &= !(1u64 << i);

        // SAFETY: The index `i` is obtained from the mask, which is guaranteed
        // to have bits set only for valid slot indices. The `slots_ptr` points
        // to a valid memory block of `NUM_SLOTS` slots.
        let slot = unsafe { &mut *self.slots_ptr.add(i) };
        Some((i, slot))
    }
}

/// Error type for slot operations.
#[derive(Debug, thiserror::Error)]
pub enum SlotError {
    /// The index is out of bounds.
    #[error("index out of bounds: {0}")]
    OutOfBounds(usize),
}
