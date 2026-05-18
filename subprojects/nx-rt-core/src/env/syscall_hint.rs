//! Syscall availability hints.

/// Syscall availability hints (192 bits for SVCs 0x00-0xBF).
///
/// Each bit represents a syscall: bit 0 = SVC 0, bit 1 = SVC 1, etc.
#[derive(Clone, Copy, Debug, Default)]
pub struct SyscallHints([u64; 3]);

impl SyscallHints {
    /// Creates a new empty hints (no syscalls available).
    pub const fn new() -> Self {
        Self([0; 3])
    }

    /// Creates hints with all syscalls marked as available.
    pub const fn all_available() -> Self {
        Self([u64::MAX, u64::MAX, u64::MAX])
    }

    /// Sets hints for SVCs 0x00-0x7F (first 128 syscalls).
    pub fn set_hint_0_7f(&mut self, low: u64, high: u64) {
        self.0[0] = low;
        self.0[1] = high;
    }

    /// Sets hints for SVCs 0x80-0xBF (syscalls 128-191).
    pub fn set_hint_80_bf(&mut self, value: u64) {
        self.0[2] = value;
    }

    /// Returns true if the given syscall is hinted as available.
    pub const fn is_available(&self, svc: u32) -> bool {
        if svc >= 192 {
            return false;
        }

        let hint_index = (svc / 64) as usize;
        let bit_index = svc % 64;
        (self.0[hint_index] & (1u64 << bit_index)) != 0
    }
}
