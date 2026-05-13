//! Capture MTP wire-layout types.

use static_assertions::const_assert_eq;

/// Wire-layout input for the session `Open` command.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct SessionOpenIn {
    pub tmem_size: u32,
    pub folder_count: u32,
    pub max_images: u32,
    pub max_videos: u32,
}

const_assert_eq!(size_of::<SessionOpenIn>(), 0x10);
