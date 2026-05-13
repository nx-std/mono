//! Wire-layout types for the RO service.

use static_assertions::const_assert_eq;

/// Module information returned by `ro:dmnt` `GetProcessModuleInfo`.
///
/// Each entry describes a loaded module's build ID, base address, and size.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LoaderModuleInfo {
    pub build_id: [u8; 0x20],
    pub base_address: u64,
    pub size: u64,
}

const_assert_eq!(core::mem::size_of::<LoaderModuleInfo>(), 0x30);

/// Input for `LoadNro` (cmd 0).
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct LoadNroIn {
    pub pid_placeholder: u64,
    pub nro_address: u64,
    pub nro_size: u64,
    pub bss_address: u64,
    pub bss_size: u64,
}

const_assert_eq!(core::mem::size_of::<LoadNroIn>(), 0x28);

/// Input for `UnloadNro` (cmd 1) and `UnloadNrr` (cmd 3).
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct UnloadIn {
    pub pid_placeholder: u64,
    pub address: u64,
}

const_assert_eq!(core::mem::size_of::<UnloadIn>(), 0x10);

/// Input for `LoadNrr` (cmd 2) and `LoadNrrEx` (cmd 10).
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct LoadNrrIn {
    pub pid_placeholder: u64,
    pub nrr_address: u64,
    pub nrr_size: u64,
}

const_assert_eq!(core::mem::size_of::<LoadNrrIn>(), 0x18);
