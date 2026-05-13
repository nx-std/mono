//! AV module wire-layout types.

use static_assertions::const_assert_eq;

/// Version list entry returned by the AVM service.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AvmVersionListEntry {
    pub application_id: u64,
    pub version: u32,
    pub required: u32,
}

const_assert_eq!(size_of::<AvmVersionListEntry>(), 0x10);

/// Required-version entry returned by the AVM service.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AvmRequiredVersionEntry {
    pub application_id: u64,
    pub version: u32,
}

const_assert_eq!(size_of::<AvmRequiredVersionEntry>(), 0x10);

/// Wire-layout input for `GetHighestAvailableVersion` / `GetHighestRequiredVersion`.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetVersionIn {
    pub id_1: u64,
    pub id_2: u64,
}

const_assert_eq!(size_of::<GetVersionIn>(), 0x10);

/// Wire-layout input for `UpgradeLaunchRequiredVersion` / `PushLaunchVersion`.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct PushVersionIn {
    pub version: u32,
    pub application_id: u64,
}

const_assert_eq!(size_of::<PushVersionIn>(), 0x10);
