//! Syscall availability hints.

/// Syscall availability hints (192 bits for SVCs 0x00-0xBF).
///
/// Each bit represents a syscall: bit 0 = SVC 0, bit 1 = SVC 1, etc.
#[derive(Debug, Clone, Copy, Default)]
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

    /// Sets hints for SVCs 0x00-0x3F (the first 64 syscalls).
    ///
    /// The three ranges are set one at a time rather than in pairs: the words
    /// are indistinguishable to the compiler, so a caller that swapped two of
    /// them would mark the wrong 64 syscalls available and nothing would fail
    /// until one of them was issued.
    pub fn set_hints_0_3f(&mut self, bits: u64) {
        self.0[0] = bits;
    }

    /// Sets hints for SVCs 0x40-0x7F (syscalls 64-127).
    pub fn set_hints_40_7f(&mut self, bits: u64) {
        self.0[1] = bits;
    }

    /// Sets hints for SVCs 0x80-0xBF (syscalls 128-191).
    pub fn set_hints_80_bf(&mut self, bits: u64) {
        self.0[2] = bits;
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
