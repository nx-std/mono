//! Data cache maintenance.

use core::arch::asm;

/// Cleans and invalidates the data cache over `[start, start + len)`.
///
/// The range is widened to whole cache lines, so a buffer that starts or ends
/// mid-line is still written back in full. The line size comes from `CTR_EL0`
/// rather than being assumed, because it is a property of the part rather than
/// of the architecture.
///
/// Call this before handing a buffer to a device that reads memory without
/// going through the CPU's caches: until the lines are cleaned, the bytes the
/// device reads are whatever main memory held before the CPU wrote to it.
///
/// # Safety
///
/// `start` must point to `len` bytes mapped by this process. Cache maintenance
/// names an address like any other access, so an unmapped one faults.
#[inline]
pub unsafe fn flush_data_range(start: *mut u8, len: usize) {
    if len == 0 {
        return;
    }

    let end = (start as usize).wrapping_add(len);

    // SAFETY: the caller guarantees the range is mapped, which is what the
    // maintenance instructions require.
    //
    // The byte at 0x104 in the thread-local region is the kernel's, not ours:
    // it reads it to learn that a cache maintenance sequence is in flight, so
    // that a thread migrated to another core mid-loop has the sequence
    // restarted rather than leaving lines behind on the core it left. It is
    // set for exactly the span of the loop, which is why the whole sequence is
    // one asm block: splitting it would let the compiler schedule the flag
    // writes away from the loop they delimit.
    unsafe {
        asm!(
            "mrs  {tls}, tpidrro_el0",
            // Line size: CTR_EL0.DminLine holds log2 of the line in words.
            "mrs  {step}, ctr_el0",
            "lsr  {step}, {step}, #16",
            "and  {step}, {step}, #0xf",
            "mov  {mask}, #4",
            "lsl  {step}, {mask}, {step}",
            "sub  {mask}, {step}, #1",
            // Round the start down so the first partial line is covered.
            "bic  {cur}, {cur}, {mask}",
            "mov  {flag:w}, #1",
            "strb {flag:w}, [{tls}, #0x104]",
            "2:",
            "dc   civac, {cur}",
            "add  {cur}, {cur}, {step}",
            "cmp  {cur}, {end}",
            "b.cc 2b",
            "dsb  sy",
            "strb wzr, [{tls}, #0x104]",
            cur = inout(reg) start as usize => _,
            end = in(reg) end,
            tls = out(reg) _,
            step = out(reg) _,
            mask = out(reg) _,
            flag = out(reg) _,
            options(nostack),
        );
    }
}
