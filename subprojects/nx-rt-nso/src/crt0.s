/* nx-rt-nso — `pm` process-launch `.crt0` startup section.
 *
 * This is the NSO process `_start` for the opt-in `rustc`-driven link
 * pipeline. It is assembled into the crate only when the `rt-link` Cargo
 * feature is enabled; on the default GCC pipeline `_start` is still supplied
 * by libnx's `switch_crt0.s`, and an unconditional `.crt0` here would produce
 * a duplicate-`_start` link error.
 *
 * It is the `rustc`-pipeline counterpart of libnx's `switch_crt0.s`: the two
 * pipelines deliberately keep separate startup sources, with this one narrowed
 * to the single output kind this crate serves.
 *
 * Startup ABI — `pm` process-launch handoff:
 *   x0 = 0
 *   x1 = main-thread handle
 * Kernel-delivered user-mode exception entry reuses the same entry point:
 *   x0 = exception type (non-zero)   x1 = pointer to the exception context
 *
 * The `.crt0` mechanism — the `_start` header, the relative MOD0 pointer,
 * `__nx_dynamic` self-relocation and BSS zeroing — is identical to the
 * homebrew-NRO kind; only the entry ABI above differs. The NSO-vs-NRO fork
 * happens later, inside `__libnx_init`, which inspects `x1`: a main-thread
 * handle (any value other than the NRO `-1` sentinel) selects the NSO path,
 * where the loader return address is the process-exit syscall. The
 * `"HOMEBREW"` magic and `"LNY*"` MOD0 blocks are libnx homebrew extensions,
 * kept here because nx-rt builds homebrew NSOs (matching `switch_crt0.s`).
 *
 * After self-relocation the `.crt0` hands off to the kind-agnostic runtime
 * init (`__libnx_init`), which drives nx-rt-core's heap / main-thread-TLS /
 * environment setup through the libnx symbol overrides.
 */

.section .crt0, "ax", %progbits
.global _start
.align 2

_start:
    b 1f
    .word __nx_mod0 - _start
    .ascii "HOMEBREW"

.org _start+0x80; 1:
    /* Route a kernel-delivered user-mode exception away from normal launch:
     * if (x0 != 0 && x1 != UINT64_MAX) __libnx_exception_entry(<inargs>). */
    cmp  x0, #0
    ccmn x1, #1, #4, ne // 4 = Z
    beq  .Lcrt0_main_entry
    b    __libnx_exception_entry

.Lcrt0_main_entry:
    // Preserve the loader-supplied state across the init calls.
    mov x25, x0  // entry-point argument 0 (0 on `pm` launch)
    mov x26, x1  // entry-point argument 1 (main-thread handle on `pm` launch)
    mov x27, x30 // loader return address
    mov x28, sp  // initial stack pointer

    // Self-relocate: apply the dynamic relocations against our own image.
    adr  x0, _start    // ASLR base
    adr  x1, __nx_mod0 // MOD0 descriptor
    bl   __nx_dynamic

    // Save the initial stack pointer for `__nx_exit`.
    adrp x9, __stack_top
    str  x28, [x9, #:lo12:__stack_top]

    // Hand off to the kind-agnostic runtime init.
    mov  x0, x25
    mov  x1, x26
    mov  x2, x27
    bl   __libnx_init

    // Enter `main`, with `exit` as the return address.
    adrp x0, __system_argc // argc
    ldr  w0, [x0, #:lo12:__system_argc]
    adrp x1, __system_argv // argv
    ldr  x1, [x1, #:lo12:__system_argv]
    adrp x30, :got:exit
    ldr  x30, [x30, #:got_lo12:exit]
    b    main

.global __nx_exit
.type   __nx_exit, %function
__nx_exit:
    // Restore the loader's stack pointer and branch back to it.
    adrp x8, __stack_top
    ldr  x8, [x8, #:lo12:__stack_top]
    mov  sp, x8
    br   x1

.global __nx_mod0
__nx_mod0:
    .ascii "MOD0"
    .word  _DYNAMIC             - __nx_mod0
    .word  __bss_start__        - __nx_mod0
    .word  __bss_end__          - __nx_mod0
    .word  __eh_frame_hdr_start - __nx_mod0
    .word  __eh_frame_hdr_end   - __nx_mod0
    .word  0 // "offset to runtime-generated module object" (unused in homebrew)

    // MOD0 extensions for homebrew
    .ascii "LNY0"
    .word  __got_start__        - __nx_mod0
    .word  __got_end__          - __nx_mod0

    .ascii "LNY1"
    .word  __relro_start        - __nx_mod0
    .word  __data_start         - __nx_mod0

    .ascii "LNY2"
    .word  0x1 // Version/Fix field, increment on recompile-the-worlds as needed
    .word  0x0 // Reserved

.section .bss.__stack_top, "aw", %nobits
.global __stack_top
.align 3

__stack_top:
    .space 8
