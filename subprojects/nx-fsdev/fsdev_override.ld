/* Linker script — libnx fsdev overrides.
 *
 * Redirects the devoptab layer that sits between the C standard library and
 * `fsp-srv` to the Rust implementations in nx-fsdev.
 *
 * Replacement is per translation unit: every global that fs_dev.o defines and
 * something outside it references must be claimed here, or the linker pulls the
 * C member back in and its definitions collide with these aliases.
 *
 * Two globals are deliberately left unclaimed: `__nx_fsdev_direntry_cache_size`
 * and `__nx_fsdev_support_cwd`. Both are weak defaults a program may override,
 * both are read only by fs_dev.c itself, and nothing this crate emits reads
 * either. Claiming them would turn a user-overridable weak symbol into a strong
 * one; leaving them alone keeps fs_dev.o unreferenced instead.
 *
 * Organization:
 *   - Rust FFI:  subprojects/nx-fsdev/src/ffi/libnx.rs
 *   - libnx C:   subprojects/libnx/src/nx/source/runtime/devices/fs_dev.c
 *
 * Note that fs_dev.o is still pulled out of the archive, because init.o
 * references `fsdevMountSdmc`. What keeps that harmless is that a script
 * assignment overrides the object's own definition, so every reference lands
 * here and section garbage collection drops the C code behind it.
 */

/*
 * Mounting
 * Rust: ffi/libnx.rs
 */
EXTERN(__nx_fsdev__libnx_fsdev_mount_sdmc);
EXTERN(__nx_fsdev__libnx_fsdev_mount_device);
EXTERN(__nx_fsdev__libnx_fsdev_unmount_device);
EXTERN(__nx_fsdev__libnx_fsdev_unmount_all);
EXTERN(__nx_fsdev__libnx_fsdev_commit_device);

fsdevMountSdmc      = __nx_fsdev__libnx_fsdev_mount_sdmc;
fsdevMountDevice    = __nx_fsdev__libnx_fsdev_mount_device;
fsdevUnmountDevice  = __nx_fsdev__libnx_fsdev_unmount_device;
fsdevUnmountAll     = __nx_fsdev__libnx_fsdev_unmount_all;
fsdevCommitDevice   = __nx_fsdev__libnx_fsdev_commit_device;

/*
 * Save-data mounts — aliased to panicking stubs. A command left to libnx runs
 * against a `g_fsSrv` this workspace zeroed, which parks forever rather than
 * failing, so an unimplemented command must still be ours.
 */
EXTERN(__nx_fsdev__libnx_fsdev_mount_save_data);
EXTERN(__nx_fsdev__libnx_fsdev_mount_save_data_read_only);
EXTERN(__nx_fsdev__libnx_fsdev_mount_bcat_save_data);
EXTERN(__nx_fsdev__libnx_fsdev_mount_device_save_data);
EXTERN(__nx_fsdev__libnx_fsdev_mount_temporary_storage);
EXTERN(__nx_fsdev__libnx_fsdev_mount_cache_storage);
EXTERN(__nx_fsdev__libnx_fsdev_mount_system_save_data);
EXTERN(__nx_fsdev__libnx_fsdev_mount_system_bcat_save_data);

fsdevMountSaveData          = __nx_fsdev__libnx_fsdev_mount_save_data;
fsdevMountSaveDataReadOnly  = __nx_fsdev__libnx_fsdev_mount_save_data_read_only;
fsdevMountBcatSaveData      = __nx_fsdev__libnx_fsdev_mount_bcat_save_data;
fsdevMountDeviceSaveData    = __nx_fsdev__libnx_fsdev_mount_device_save_data;
fsdevMountTemporaryStorage  = __nx_fsdev__libnx_fsdev_mount_temporary_storage;
fsdevMountCacheStorage      = __nx_fsdev__libnx_fsdev_mount_cache_storage;
fsdevMountSystemSaveData    = __nx_fsdev__libnx_fsdev_mount_system_save_data;
fsdevMountSystemBcatSaveData = __nx_fsdev__libnx_fsdev_mount_system_bcat_save_data;

/*
 * Path and entry operations
 */
EXTERN(__nx_fsdev__libnx_fsdev_get_device_file_system);
EXTERN(__nx_fsdev__libnx_fsdev_translate_path);
EXTERN(__nx_fsdev__libnx_fsdev_create_file);
EXTERN(__nx_fsdev__libnx_fsdev_delete_directory_recursively);
EXTERN(__nx_fsdev__libnx_fsdev_set_concatenation_file_attribute);
EXTERN(__nx_fsdev__libnx_fsdev_is_valid_signed_system_partition_on_sd_card);
EXTERN(__nx_fsdev__libnx_fsdev_get_last_result);

fsdevGetDeviceFileSystem = __nx_fsdev__libnx_fsdev_get_device_file_system;
fsdevTranslatePath       = __nx_fsdev__libnx_fsdev_translate_path;
fsdevCreateFile          = __nx_fsdev__libnx_fsdev_create_file;
fsdevDeleteDirectoryRecursively = __nx_fsdev__libnx_fsdev_delete_directory_recursively;
fsdevSetConcatenationFileAttribute = __nx_fsdev__libnx_fsdev_set_concatenation_file_attribute;
fsdevIsValidSignedSystemPartitionOnSdCard = __nx_fsdev__libnx_fsdev_is_valid_signed_system_partition_on_sd_card;
fsdevGetLastResult       = __nx_fsdev__libnx_fsdev_get_last_result;

/*
 * Startup working directory
 * libnx: runtime/devices/fs_dev.c
 */
EXTERN(__nx_fsdev__libnx_init_cwd);
__libnx_init_cwd = __nx_fsdev__libnx_init_cwd;
