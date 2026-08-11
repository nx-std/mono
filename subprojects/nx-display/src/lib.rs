//! # nx-display
//!
//! High-level display primitives built on `nx-service-vi`'s IPC transport.
//!
//! This crate hosts user-facing types that compose the IGBP wrapper, `Binder`,
//! and `Parcel` from `nx-service-vi` into ergonomic surfaces:
//!
//! - [`NativeWindow`] — mirrors libnx `NWindow` (`display/native_window.h`):
//!   a stateful producer-side window that owns a binder session, frames'
//!   slot table, and dimensions/crop/transform/swap-interval configuration.
//!
//! Framebuffer support (libnx `display/framebuffer.h`) is intentionally
//! deferred until the NV graphic-buffer port lands in `nx-service-nv` — see
//! the plan at `plans/assess-the-subprojects-libnx-vi-compiled-stardust.md`
//! for the staged sequencing.
//!
//! ## Layering
//!
//! ```text
//!   nx-display          NativeWindow (and Framebuffer, eventually)
//!         |
//!   nx-service-vi       IGBP wrapper · Binder · Parcel
//!         |
//!     libnx VI IPC      vi:m/vi:s/vi:u and IHOSBinderDriverRelay
//! ```

#![no_std]

extern crate alloc;
extern crate nx_alloc; // proves a #[global_allocator] exists at link time
extern crate nx_panic_handler; // provides #[panic_handler]

pub mod framebuffer;
pub mod native_window;

pub use self::native_window::{
    NATIVE_WINDOW_API_CAMERA,
    NATIVE_WINDOW_API_CPU,
    NATIVE_WINDOW_API_EGL,
    NATIVE_WINDOW_API_MEDIA,
    NativeWindow,
    NativeWindowApi,
    NativeWindowError,
    Transform,
};
