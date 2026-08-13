/* Linker script — libnx romfs overrides.
 *
 * Redirects the read-only device that sits between the C standard library and
 * a romfs image to the Rust implementations in nx-romfs.
 *
 * Replacement is per translation unit: every global that romfs_dev.o defines
 * and something outside it references must be claimed here, or the linker pulls
 * the C member back in and its definitions collide with these aliases.
 *
 * `romfsMountSelf` is deliberately absent. It is the one entry point that has
 * to know whether the process is an NRO or an NSO before it can pick a source,
 * and that question is answered by which runtime crate the binary links rather
 * than by asking at run time. Its alias therefore lives in the per-kind
 * fragments, `rt_nro_libnx_romfs.ld` and `rt_nso_libnx_romfs.ld`, each pointing
 * at the symbol its own entry crate defines. Exactly one of those fragments is
 * ever in a link, so the two definitions cannot collide.
 *
 * `romfsInit` and `romfsExit` are absent for a different reason: both are
 * `static inline` in the libnx header, so they are compiled into each caller
 * and there is no symbol to redirect. A caller reaching `romfsInit` lands on
 * `romfsMountSelf` above.
 *
 * `__romfs_path`, the weak override a program may define to name its own NRO,
 * is left unclaimed. Nothing this crate emits reads it: the path comes from the
 * command line, which the runtime installs in `nx-sys-args` below every caller.
 * Claiming it would turn a user-overridable weak symbol into a strong one.
 *
 * Organization:
 *   - Rust FFI:  subprojects/nx-romfs/src/ffi/libnx.rs
 *   - libnx C:   subprojects/libnx/src/nx/source/runtime/devices/romfs_dev.c
 */

/*
 * Mounting
 * Rust: ffi/libnx.rs
 */
EXTERN(__nx_romfs__libnx_romfs_mount_from_file);
EXTERN(__nx_romfs__libnx_romfs_mount_from_storage);
EXTERN(__nx_romfs__libnx_romfs_mount_from_current_process);
EXTERN(__nx_romfs__libnx_romfs_mount_data_storage_from_program);
EXTERN(__nx_romfs__libnx_romfs_mount_from_fsdev);
EXTERN(__nx_romfs__libnx_romfs_mount_from_data_archive);
EXTERN(__nx_romfs__libnx_romfs_unmount);

romfsMountFromFile              = __nx_romfs__libnx_romfs_mount_from_file;
romfsMountFromStorage           = __nx_romfs__libnx_romfs_mount_from_storage;
romfsMountFromCurrentProcess    = __nx_romfs__libnx_romfs_mount_from_current_process;
romfsMountDataStorageFromProgram = __nx_romfs__libnx_romfs_mount_data_storage_from_program;
romfsMountFromFsdev             = __nx_romfs__libnx_romfs_mount_from_fsdev;
romfsMountFromDataArchive       = __nx_romfs__libnx_romfs_mount_from_data_archive;
romfsUnmount                    = __nx_romfs__libnx_romfs_unmount;
