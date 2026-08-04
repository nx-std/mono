/*
 * Filesystem (fsp-srv) API
 * Rust: ffi/libnx/fs.rs
 * libnx: services/fs.c
 *
 * `g_fsSrv` and `g_fsSessionMgr` are `static` in fs.c, so no alias can reach
 * them. This fragment instead redirects every `fs*` entry point fsdev calls,
 * which is what keeps the C globals from ever being read. Leaving one out
 * would send a non-domain request against a domain object id.
 */
EXTERN(__nx_rt_nro__libnx_fs_initialize);
EXTERN(__nx_rt_nro__libnx_fs_exit);
EXTERN(__nx_rt_nro__libnx_fs_get_service_session);
EXTERN(__nx_rt_nro__libnx_fs_set_priority);
EXTERN(__nx_rt_nro__libnx_fs_open_sd_card_file_system);

fsInitialize = __nx_rt_nro__libnx_fs_initialize;
fsExit = __nx_rt_nro__libnx_fs_exit;
fsGetServiceSession = __nx_rt_nro__libnx_fs_get_service_session;
fsSetPriority = __nx_rt_nro__libnx_fs_set_priority;
fsOpenSdCardFileSystem = __nx_rt_nro__libnx_fs_open_sd_card_file_system;

/*
 * IFileSystem commands
 * Rust: ffi/libnx/fs.rs
 * libnx: services/fs.c
 */
EXTERN(__nx_rt_nro__libnx_fs_fs_create_file);
EXTERN(__nx_rt_nro__libnx_fs_fs_delete_file);
EXTERN(__nx_rt_nro__libnx_fs_fs_create_directory);
EXTERN(__nx_rt_nro__libnx_fs_fs_delete_directory);
EXTERN(__nx_rt_nro__libnx_fs_fs_delete_directory_recursively);
EXTERN(__nx_rt_nro__libnx_fs_fs_clean_directory_recursively);
EXTERN(__nx_rt_nro__libnx_fs_fs_rename_file);
EXTERN(__nx_rt_nro__libnx_fs_fs_rename_directory);
EXTERN(__nx_rt_nro__libnx_fs_fs_get_entry_type);
EXTERN(__nx_rt_nro__libnx_fs_fs_open_file);
EXTERN(__nx_rt_nro__libnx_fs_fs_open_directory);
EXTERN(__nx_rt_nro__libnx_fs_fs_commit);
EXTERN(__nx_rt_nro__libnx_fs_fs_get_free_space);
EXTERN(__nx_rt_nro__libnx_fs_fs_get_total_space);
EXTERN(__nx_rt_nro__libnx_fs_fs_get_file_time_stamp_raw);
EXTERN(__nx_rt_nro__libnx_fs_fs_set_concatenation_file_attribute);
EXTERN(__nx_rt_nro__libnx_fs_fs_is_valid_signed_system_partition_on_sd_card);
EXTERN(__nx_rt_nro__libnx_fs_fs_close);

fsFsCreateFile = __nx_rt_nro__libnx_fs_fs_create_file;
fsFsDeleteFile = __nx_rt_nro__libnx_fs_fs_delete_file;
fsFsCreateDirectory = __nx_rt_nro__libnx_fs_fs_create_directory;
fsFsDeleteDirectory = __nx_rt_nro__libnx_fs_fs_delete_directory;
fsFsDeleteDirectoryRecursively = __nx_rt_nro__libnx_fs_fs_delete_directory_recursively;
fsFsCleanDirectoryRecursively = __nx_rt_nro__libnx_fs_fs_clean_directory_recursively;
fsFsRenameFile = __nx_rt_nro__libnx_fs_fs_rename_file;
fsFsRenameDirectory = __nx_rt_nro__libnx_fs_fs_rename_directory;
fsFsGetEntryType = __nx_rt_nro__libnx_fs_fs_get_entry_type;
fsFsOpenFile = __nx_rt_nro__libnx_fs_fs_open_file;
fsFsOpenDirectory = __nx_rt_nro__libnx_fs_fs_open_directory;
fsFsCommit = __nx_rt_nro__libnx_fs_fs_commit;
fsFsGetFreeSpace = __nx_rt_nro__libnx_fs_fs_get_free_space;
fsFsGetTotalSpace = __nx_rt_nro__libnx_fs_fs_get_total_space;
fsFsGetFileTimeStampRaw = __nx_rt_nro__libnx_fs_fs_get_file_time_stamp_raw;
fsFsSetConcatenationFileAttribute = __nx_rt_nro__libnx_fs_fs_set_concatenation_file_attribute;
fsFsIsValidSignedSystemPartitionOnSdCard = __nx_rt_nro__libnx_fs_fs_is_valid_signed_system_partition_on_sd_card;
fsFsClose = __nx_rt_nro__libnx_fs_fs_close;

/*
 * IFile commands
 * Rust: ffi/libnx/fs.rs
 * libnx: services/fs.c
 */
EXTERN(__nx_rt_nro__libnx_fs_file_read);
EXTERN(__nx_rt_nro__libnx_fs_file_write);
EXTERN(__nx_rt_nro__libnx_fs_file_flush);
EXTERN(__nx_rt_nro__libnx_fs_file_set_size);
EXTERN(__nx_rt_nro__libnx_fs_file_get_size);
EXTERN(__nx_rt_nro__libnx_fs_file_close);

fsFileRead = __nx_rt_nro__libnx_fs_file_read;
fsFileWrite = __nx_rt_nro__libnx_fs_file_write;
fsFileFlush = __nx_rt_nro__libnx_fs_file_flush;
fsFileSetSize = __nx_rt_nro__libnx_fs_file_set_size;
fsFileGetSize = __nx_rt_nro__libnx_fs_file_get_size;
fsFileClose = __nx_rt_nro__libnx_fs_file_close;

/*
 * IDirectory commands
 * Rust: ffi/libnx/fs.rs
 * libnx: services/fs.c
 */
EXTERN(__nx_rt_nro__libnx_fs_dir_read);
EXTERN(__nx_rt_nro__libnx_fs_dir_get_entry_count);
EXTERN(__nx_rt_nro__libnx_fs_dir_close);

fsDirRead = __nx_rt_nro__libnx_fs_dir_read;
fsDirGetEntryCount = __nx_rt_nro__libnx_fs_dir_get_entry_count;
fsDirClose = __nx_rt_nro__libnx_fs_dir_close;
