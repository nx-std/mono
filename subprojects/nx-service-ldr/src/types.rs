//! Wire-layout types for the loader service.

use static_assertions::const_assert_eq;

/// Module information returned by `ldr:dmnt` `GetProcessModuleInfo`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LoaderModuleInfo {
    pub build_id: [u8; 0x20],
    pub base_address: u64,
    pub size: u64,
}

const_assert_eq!(core::mem::size_of::<LoaderModuleInfo>(), 0x30);

/// Program attributes for `ldr:pm` commands (`[20.0.0+/Atmosphere]`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LoaderProgramAttributes {
    pub platform: u8,
    pub content_attributes: u8,
}

const_assert_eq!(core::mem::size_of::<LoaderProgramAttributes>(), 0x2);

/// Program location identifying a program by ID and storage.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcmProgramLocation {
    pub program_id: u64,
    pub storage_id: u8,
    pub pad: [u8; 7],
}

const_assert_eq!(core::mem::size_of::<NcmProgramLocation>(), 0x10);

/// Program info returned by `ldr:pm` `GetProgramInfo` (`[1.0.0–18.1.0]`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LoaderProgramInfoV1 {
    pub main_thread_priority: u8,
    pub default_cpu_id: u8,
    pub application_type: u16,
    pub main_thread_stack_size: u32,
    pub program_id: u64,
    pub acid_sac_size: u32,
    pub aci0_sac_size: u32,
    pub acid_fac_size: u32,
    pub aci0_fah_size: u32,
    pub ac_buffer: [u8; 0x3E0],
}

const_assert_eq!(core::mem::size_of::<LoaderProgramInfoV1>(), 0x400);

/// Program info returned by `ldr:pm` `GetProgramInfo` (`[19.0.0+/Atmosphere]`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LoaderProgramInfo {
    pub main_thread_priority: u8,
    pub default_cpu_id: u8,
    pub application_type: u16,
    pub main_thread_stack_size: u32,
    pub program_id: u64,
    pub acid_sac_size: u32,
    pub aci0_sac_size: u32,
    pub acid_fac_size: u32,
    pub aci0_fah_size: u32,
    pub unused: [u8; 0x10],
    pub ac_buffer: [u8; 0x3E0],
}

const_assert_eq!(core::mem::size_of::<LoaderProgramInfo>(), 0x410);

/// Input for legacy `SetProgramArguments` (pre-11.0.0).
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct SetProgramArgumentsLegacyIn {
    pub args_size: u32,
    pub pad: u32,
    pub program_id: u64,
}

const_assert_eq!(core::mem::size_of::<SetProgramArgumentsLegacyIn>(), 0x10);

/// Input for `CreateProcess` (pre-20.0.0, no attributes).
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct CreateProcessLegacyIn {
    pub flags: u32,
    pub pad: u32,
    pub pin_id: u64,
}

const_assert_eq!(core::mem::size_of::<CreateProcessLegacyIn>(), 0x10);

/// Input for `CreateProcess` (`[20.0.0+/Atmosphere]`, with attributes).
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct CreateProcessIn {
    pub attr: LoaderProgramAttributes,
    pub pad: u16,
    pub flags: u32,
    pub pin_id: u64,
}

const_assert_eq!(core::mem::size_of::<CreateProcessIn>(), 0x10);

/// Input for `GetProgramInfo` (`[20.0.0+/Atmosphere]`, with attributes).
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct GetProgramInfoIn {
    pub attr: LoaderProgramAttributes,
    pub pad1: u16,
    pub pad2: u32,
    pub loc: NcmProgramLocation,
}

const_assert_eq!(core::mem::size_of::<GetProgramInfoIn>(), 0x18);
