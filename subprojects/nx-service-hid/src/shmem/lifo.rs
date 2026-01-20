//! Lock-free LIFO ring buffer reader for HID shared memory.
//!
//! The HID service uses atomic LIFO (Last In, First Out) ring buffers to share
//! input state with applications. This module implements the lock-free reading
//! algorithm with consistency checks.

use core::{
    ptr,
    sync::atomic::{AtomicU64, Ordering},
};

use super::types::InputState;

/// Common LIFO header for all HID input types.
///
/// This structure is at the start of each LIFO buffer in shared memory.
#[repr(C)]
pub struct HidCommonLifoHeader {
    pub unused: u64,
    pub buffer_count: u64,
    pub tail: AtomicU64,
    pub count: AtomicU64,
}

/// Read states from a LIFO ring buffer with atomic consistency guarantees.
///
/// This function implements the lock-free algorithm used by libnx:
/// 1. Atomically load tail and count
/// 2. Calculate position: (tail + max_states + 1 - count + i) % max_states
/// 3. Read from oldest to newest, output in reverse (newest first)
/// 4. Check sampling numbers for consistency (torn reads and sequential order)
/// 5. Retry if inconsistent
///
/// # Arguments
///
/// * `header` - LIFO header containing tail and count
/// * `storage` - Ring buffer storage array
/// * `out` - Output buffer for states (most recent first)
///
/// # Returns
///
/// Number of states read (may be less than `out.len()`)
///
/// # Safety
///
/// Caller must ensure `header` and `storage` point to valid shared memory.
pub fn get_states<T: InputState>(
    header: &HidCommonLifoHeader,
    storage: &[T::Storage],
    out: &mut [T],
) -> usize {
    const MAX_RETRIES: usize = 3;
    let max_states = storage.len() as u64;

    for _ in 0..MAX_RETRIES {
        // Atomically load tail and count
        let tail = header.tail.load(Ordering::Acquire);
        let count = header
            .count
            .load(Ordering::Acquire)
            .min(header.buffer_count);

        let total_entries = count.min(out.len() as u64).min(max_states);
        if total_entries == 0 {
            return 0;
        }

        let mut consistent = true;
        let mut prev_sampling = 0u64;

        // Read from oldest to newest
        for i in 0..total_entries {
            // libnx formula: (tail + max_states + 1 - total_entries + i) % max_states
            let entrypos = ((tail + max_states + 1 - total_entries) + i) % max_states;

            // Load with torn-read detection
            // Safety: entrypos is bounds-checked against storage.len()
            let entry = &storage[entrypos as usize];

            // Read sampling number before and after loading state
            let sampling0 = unsafe { ptr::read_volatile(entry as *const T::Storage as *const u64) };
            let state = unsafe { T::load_from_storage(entry) };
            let sampling1 = unsafe { ptr::read_volatile(entry as *const T::Storage as *const u64) };

            let curr_sampling = state.sampling_number();

            // Output in reverse order (newest first)
            let out_idx = (total_entries - 1 - i) as usize;

            // Consistency checks:
            // 1. Torn read check: sampling numbers changed during read
            // 2. Sequential check: adjacent samples should differ by exactly 1
            if sampling0 != sampling1 {
                consistent = false;
                break;
            }

            if i > 0 && prev_sampling.wrapping_sub(curr_sampling) != 1 {
                consistent = false;
                break;
            }

            out[out_idx] = state;
            prev_sampling = curr_sampling;
        }

        if consistent {
            return total_entries as usize;
        }

        // Inconsistent read, retry
    }

    // Failed to get consistent read after retries
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    struct TestState {
        sampling_number: u64,
        value: i32,
    }

    impl InputState for TestState {
        type Storage = TestState;

        fn sampling_number(&self) -> u64 {
            self.sampling_number
        }

        unsafe fn load_from_storage(storage: &Self::Storage) -> Self {
            *storage
        }
    }

    #[test]
    fn test_get_states_empty() {
        let header = HidCommonLifoHeader {
            unused: 0,
            buffer_count: 17,
            tail: AtomicU64::new(0),
            count: AtomicU64::new(0),
        };

        let storage = [TestState {
            sampling_number: 0,
            value: 0,
        }; 17];

        let mut out = [TestState {
            sampling_number: 0,
            value: 0,
        }; 5];

        let count = get_states(&header, &storage, &mut out);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_get_states_single() {
        let header = HidCommonLifoHeader {
            unused: 0,
            buffer_count: 17,
            tail: AtomicU64::new(0),
            count: AtomicU64::new(1),
        };

        let mut storage = [TestState {
            sampling_number: 0,
            value: 0,
        }; 17];

        storage[0] = TestState {
            sampling_number: 100,
            value: 42,
        };

        let mut out = [TestState {
            sampling_number: 0,
            value: 0,
        }; 5];

        let count = get_states(&header, &storage, &mut out);
        assert_eq!(count, 1);
        assert_eq!(out[0].sampling_number, 100);
        assert_eq!(out[0].value, 42);
    }
}
