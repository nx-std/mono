/* Linker script — libnx romfsMountSelf override, homebrew-NRO arm.
 *
 * The rest of the `romfs*` surface is redirected by
 * `subprojects/nx-romfs/romfs_override.ld`. This one entry point is not there
 * because it is the only one that has to know what kind of program is asking:
 * an NRO reads the image appended to its own file, an NSO mounts its data
 * partition. That question is answered by which entry crate a binary links, so
 * each entry crate defines the symbol over its own source and claims the C name
 * in a fragment of its own.
 *
 * Exactly one entry crate is in any link, so this fragment and
 * `rt_nso_libnx_romfs.ld` never appear together and the two assignments cannot
 * collide.
 *
 * `romfsInit` and `romfsExit` are `static inline` in the libnx header, so they
 * are compiled into each caller and have no symbol to redirect. A caller
 * reaching `romfsInit` lands here.
 *
 * Organization:
 *   - Rust FFI:  subprojects/nx-rt-nro/src/ffi/libnx/romfs.rs
 *   - libnx C:   subprojects/libnx/src/nx/source/runtime/devices/romfs_dev.c
 */

EXTERN(__nx_rt_nro__libnx_romfs_mount_self);

romfsMountSelf = __nx_rt_nro__libnx_romfs_mount_self;
