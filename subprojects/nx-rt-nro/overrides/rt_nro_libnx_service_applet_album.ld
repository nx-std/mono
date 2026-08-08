/*
 * Album applet (`photoViewer` library applet) API
 * Rust: ffi/libnx/album.rs
 * libnx: applets/album_la.c
 *
 * Covers every `albumLa*` entry point. libnx reaches the applet through
 * `g_appletILibraryAppletCreator`, which is `static` in applet.c and cannot be
 * aliased, so once `appletInitialize` is overridden no `albumLa*` function left
 * to libnx has a session to use. All three are ported.
 */

/* Album views */
EXTERN(__nx_rt_nro__libnx_album_la_show_album_files);
EXTERN(__nx_rt_nro__libnx_album_la_show_all_album_files);
EXTERN(__nx_rt_nro__libnx_album_la_show_all_album_files_for_home_menu);

/* Album views */
albumLaShowAlbumFiles = __nx_rt_nro__libnx_album_la_show_album_files;
albumLaShowAllAlbumFiles = __nx_rt_nro__libnx_album_la_show_all_album_files;
albumLaShowAllAlbumFilesForHomeMenu = __nx_rt_nro__libnx_album_la_show_all_album_files_for_home_menu;
