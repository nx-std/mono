//! Shared font / platform (`pl`) service implementation.
//!
//! Provides access to the system's shared font data via two service variants:
//!
//! - **`pl:u`** (user): Full shared font access including shared memory.
//! - **`pl:s`** (system): System-level access. On \[16.0.0+\], shared font
//!   commands (0–4) are no longer available via `pl:s`; use `pl:u` instead.
//!
//! ## Divergence from libnx
//!
//! libnx's `pl.c` manages a global singleton with automatic shared memory
//! mapping and a font-load wait loop. This crate exposes the raw IPC commands
//! directly — shared memory mapping, load-request retries, and range
//! verification are the caller's responsibility.

#![no_std]

extern crate nx_panic_handler;

use nx_service_sm::SmService;
use nx_sf::service::{BorrowedSessionHandle, Session};

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{
        GetLoadStateError, GetSharedFontError, GetSharedMemoryAddressOffsetError,
        GetSharedMemoryNativeHandleError, GetSizeError, RequestLoadError,
    },
    proto::{PLS_SERVICE_NAME, PLU_SERVICE_NAME},
    types::{GetSharedFontOut, PlServiceType, SharedFontType},
};

/// PL (shared font / platform) session wrapper.
///
/// Wraps a connection to either `pl:u` or `pl:s`.
#[repr(transparent)]
pub struct PlService(Session);

impl PlService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl PlService {
    /// Requests loading of a shared font into shared memory.
    #[inline]
    pub fn request_load(&self, font_type: u32) -> Result<(), RequestLoadError> {
        cmif::request_load(self.0.handle(), font_type)
    }

    /// Gets the load state of a shared font.
    ///
    /// Returns the load state: 0 = not loaded, 1 = loaded.
    #[inline]
    pub fn get_load_state(&self, font_type: u32) -> Result<u32, GetLoadStateError> {
        cmif::get_load_state(self.0.handle(), font_type)
    }

    /// Gets the size of a shared font in bytes.
    #[inline]
    pub fn get_size(&self, font_type: u32) -> Result<u32, GetSizeError> {
        cmif::get_size(self.0.handle(), font_type)
    }

    /// Gets the byte offset of a shared font within shared memory.
    #[inline]
    pub fn get_shared_memory_address_offset(
        &self,
        font_type: u32,
    ) -> Result<u32, GetSharedMemoryAddressOffsetError> {
        cmif::get_shared_memory_address_offset(self.0.handle(), font_type)
    }

    /// Gets the shared memory native handle (copy handle).
    ///
    /// The returned handle can be used with `svcMapSharedMemory` to map
    /// the font shared memory region (0x1100000 bytes, read-only).
    #[inline]
    pub fn get_shared_memory_native_handle(&self) -> Result<u32, GetSharedMemoryNativeHandleError> {
        cmif::get_shared_memory_native_handle(self.0.handle())
    }

    /// Gets shared fonts for a language code.
    ///
    /// Writes font type IDs into `types`, byte offsets into `offsets`, and
    /// byte sizes into `sizes`. All three buffers must have the same length
    /// (typically [`SharedFontType::TOTAL`] elements).
    ///
    /// Returns [`GetSharedFontOut`] indicating whether fonts are loaded and
    /// how many entries were written.
    #[inline]
    pub fn get_shared_font(
        &self,
        language_code: u64,
        types: &mut [u32],
        offsets: &mut [u32],
        sizes: &mut [u32],
    ) -> Result<GetSharedFontOut, GetSharedFontError> {
        cmif::get_shared_font(self.0.handle(), language_code, types, offsets, sizes)
    }
}

/// Connects to the `pl:u` (user) service using CMIF.
pub fn connect_plu_cmif(sm: &SmService) -> Result<PlService, ConnectPluCmifError> {
    let handle = sm
        .get_service_handle_cmif(PLU_SERVICE_NAME)
        .map_err(ConnectPluCmifError)?;

    let service = Session::new(handle, 0);

    Ok(PlService(service))
}

/// Error returned by [`connect_plu_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get pl:u service")]
pub struct ConnectPluCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

/// Connects to the `pl:s` (system) service using CMIF.
pub fn connect_pls_cmif(sm: &SmService) -> Result<PlService, ConnectPlsCmifError> {
    let handle = sm
        .get_service_handle_cmif(PLS_SERVICE_NAME)
        .map_err(ConnectPlsCmifError)?;

    let service = Session::new(handle, 0);

    Ok(PlService(service))
}

/// Error returned by [`connect_pls_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get pl:s service")]
pub struct ConnectPlsCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
