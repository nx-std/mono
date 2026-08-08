//! Album applet (`photoViewer` library applet) FFI.
//!
//! libnx's `album_la.c` holds no file-local state, but every function in it
//! reaches the applet through `g_appletILibraryAppletCreator`, which is `static`
//! in `applet.c` and so cannot be aliased. Our `appletInitialize` override
//! replaces the only code that would populate it, so once `use_nx_service_applet`
//! is on, *every* libnx `albumLa*` function runs against a zeroed session.
//!
//! That is why this module covers the whole surface: a command left to libnx
//! does not fail cleanly. Here that costs nothing, because all three of libnx's
//! entry points are ported.

use nx_service_applet_album::AlbumView;
use nx_sf::error::ToResultCode as _;

use crate::{
    ffi::common::GENERIC_ERROR,
    services::applet,
};

/// Shows `view`, blocking until the user leaves the Album.
///
/// Shared by the three `albumLa*` entry points, which differ only in the view.
fn show(view: AlbumView) -> u32 {
    let (Some(self_controller), Some(creator)) = (
        applet::get_self_controller(),
        applet::get_library_applet_creator(),
    ) else {
        return GENERIC_ERROR;
    };

    match view.show(&self_controller.get(), &creator.get()) {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Shows only the files the launching application created.
///
/// Corresponds to `albumLaShowAlbumFiles()` in `album_la.h`. Blocks until the
/// user leaves the Album.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_nro__libnx_album_la_show_album_files() -> u32 {
    show(AlbumView::ApplicationFiles)
}

/// Shows every album file, with filtering allowed.
///
/// Corresponds to `albumLaShowAllAlbumFiles()` in `album_la.h`. Blocks until the
/// user leaves the Album.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_nro__libnx_album_la_show_all_album_files() -> u32 {
    show(AlbumView::AllFiles)
}

/// Shows every album file as the HOME menu does, startup sound included.
///
/// Corresponds to `albumLaShowAllAlbumFilesForHomeMenu()` in `album_la.h`.
/// Blocks until the user leaves the Album.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_nro__libnx_album_la_show_all_album_files_for_home_menu() -> u32 {
    show(AlbumView::AllFilesForHomeMenu)
}
