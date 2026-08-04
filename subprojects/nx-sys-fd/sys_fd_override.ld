/* Static linker script for file descriptor table symbols redirection */
/* Redirects libsysbase functions to nx-sys-fd __nx_sys_fd__* implementations */

/* Replacement is per translation unit: every global that handle_manager.o and
   iosupport.o define must be claimed here, or the linker pulls the C member back
   in and the definitions collide. */

/* Device registry (iosupport.o) - devoptab_list and dotab_stdnull are data, not functions */
EXTERN(__nx_sys_fd__libsysbase_devoptab_list)
EXTERN(__nx_sys_fd__libsysbase_dotab_stdnull)
EXTERN(__nx_sys_fd__libsysbase_AddDevice)
EXTERN(__nx_sys_fd__libsysbase_FindDevice)
EXTERN(__nx_sys_fd__libsysbase_RemoveDevice)
EXTERN(__nx_sys_fd__libsysbase_GetDeviceOpTab)
EXTERN(__nx_sys_fd__libsysbase_setDefaultDevice)

devoptab_list    = __nx_sys_fd__libsysbase_devoptab_list;
dotab_stdnull    = __nx_sys_fd__libsysbase_dotab_stdnull;
AddDevice        = __nx_sys_fd__libsysbase_AddDevice;
FindDevice       = __nx_sys_fd__libsysbase_FindDevice;
RemoveDevice     = __nx_sys_fd__libsysbase_RemoveDevice;
GetDeviceOpTab   = __nx_sys_fd__libsysbase_GetDeviceOpTab;
setDefaultDevice = __nx_sys_fd__libsysbase_setDefaultDevice;

/* Descriptor table (handle_manager.o) */
EXTERN(__nx_sys_fd__libsysbase_alloc_handle)
EXTERN(__nx_sys_fd__libsysbase_get_handle)
EXTERN(__nx_sys_fd__libsysbase_release_handle)
EXTERN(__nx_sys_fd__libsysbase_dup)
EXTERN(__nx_sys_fd__libsysbase_dup2)
EXTERN(__nx_sys_fd__libsysbase_close_r)

__alloc_handle   = __nx_sys_fd__libsysbase_alloc_handle;
__get_handle     = __nx_sys_fd__libsysbase_get_handle;
__release_handle = __nx_sys_fd__libsysbase_release_handle;
dup              = __nx_sys_fd__libsysbase_dup;
dup2             = __nx_sys_fd__libsysbase_dup2;
_close_r         = __nx_sys_fd__libsysbase_close_r;
