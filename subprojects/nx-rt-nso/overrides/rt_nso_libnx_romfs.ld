/* Linker script — libnx romfsMountSelf override, NSO arm.
 *
 * The NSO counterpart of `rt_hbapp_libnx_romfs.ld`; see that fragment for why this
 * one entry point is claimed per output kind rather than alongside the rest of
 * the `romfs*` surface in `subprojects/nx-romfs/romfs_override.ld`.
 *
 * Exactly one entry crate is in any link, so this fragment and the NRO one never
 * appear together and the two assignments cannot collide.
 *
 * Organization:
 *   - Rust FFI:  subprojects/nx-rt-nso/src/ffi/libnx/romfs.rs
 *   - libnx C:   subprojects/libnx/src/nx/source/runtime/devices/romfs_dev.c
 */

EXTERN(__nx_rt_nso__libnx_romfs_mount_self);

romfsMountSelf = __nx_rt_nso__libnx_romfs_mount_self;
