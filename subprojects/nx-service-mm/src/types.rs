//! Wire-layout types for the multimedia service.

use static_assertions::const_assert_eq;

/// Hardware module identifier for multimedia clock requests.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MmuModuleId {
    Ram = 2,
    Nvenc = 5,
    Nvdec = 6,
    Nvjpg = 7,
}

impl MmuModuleId {
    /// Returns the raw `u32` value of this module ID.
    #[inline]
    pub fn as_raw(self) -> u32 {
        self as u32
    }
}

/// Opaque request handle returned by
/// [`MmService::request_initialize`](crate::MmService::request_initialize) /
/// [`MmService::request_initialize_legacy`](crate::MmService::request_initialize_legacy).
///
/// Pairs the module ID with the server-assigned request ID so that
/// both legacy (pre-2.0.0, keyed by module) and modern (2.0.0+, keyed
/// by request ID) command variants can be served from the same handle.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MmuRequest {
    /// The module this request was opened for.
    pub module: MmuModuleId,
    /// Server-assigned request identifier.
    pub id: u32,
}

const_assert_eq!(size_of::<MmuRequest>(), 0x8);
