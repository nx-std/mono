//! CMIF protocol operations for the AVM service.

use core::{mem::size_of, ptr};

use nx_sf::service::{BufferAttr, DispatchError, Domain, DomainObject};

use crate::{
    dispatch::{dispatch_in, dispatch_no_io},
    proto,
    types::{AvmRequiredVersionEntry, AvmVersionListEntry, GetVersionIn, PushVersionIn},
};

/// Gets the highest available version for a title pair.
pub(crate) fn get_highest_available_version(
    domain: &Domain,
    id_1: u64,
    id_2: u64,
) -> Result<u32, DispatchError> {
    let input = GetVersionIn { id_1, id_2 };
    // SAFETY: `input` lives on the stack until `.send()` returns.
    let result = unsafe {
        domain
            .dispatch(proto::GET_HIGHEST_AVAILABLE_VERSION)
            .in_raw((&raw const input).cast::<u8>(), size_of::<GetVersionIn>())
            .out_size(size_of::<u32>())
            .send()?
    };
    // SAFETY: the response payload is at least `size_of::<u32>()` bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}

/// Gets the highest required version for a title pair.
pub(crate) fn get_highest_required_version(
    domain: &Domain,
    id_1: u64,
    id_2: u64,
) -> Result<u32, DispatchError> {
    let input = GetVersionIn { id_1, id_2 };
    // SAFETY: `input` lives on the stack until `.send()` returns.
    let result = unsafe {
        domain
            .dispatch(proto::GET_HIGHEST_REQUIRED_VERSION)
            .in_raw((&raw const input).cast::<u8>(), size_of::<GetVersionIn>())
            .out_size(size_of::<u32>())
            .send()?
    };
    // SAFETY: the response payload is at least `size_of::<u32>()` bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}

/// Gets a single version list entry by application ID.
pub(crate) fn get_version_list_entry(
    domain: &Domain,
    application_id: u64,
) -> Result<AvmVersionListEntry, DispatchError> {
    // SAFETY: `application_id` lives on the stack until `.send()` returns.
    let result = unsafe {
        domain
            .dispatch(proto::GET_VERSION_LIST_ENTRY)
            .in_raw((&raw const application_id).cast::<u8>(), size_of::<u64>())
            .out_size(size_of::<AvmVersionListEntry>())
            .send()?
    };
    // SAFETY: the response payload is at least `size_of::<AvmVersionListEntry>()` bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<AvmVersionListEntry>()) })
}

/// Gets a version list importer sub-object.
///
/// Returns the raw sub-object ID for the new importer domain object.
pub(crate) fn get_version_list_importer(
    domain: &Domain,
) -> Result<u32, GetVersionListImporterError> {
    let result = domain
        .dispatch(proto::GET_VERSION_LIST_IMPORTER)
        .out_objects(1)
        .send()
        .map_err(GetVersionListImporterError::Dispatch)?;

    if result.objects.is_empty() {
        return Err(GetVersionListImporterError::MissingObject);
    }
    Ok(result.objects[0])
}

/// Gets the launch-required version for an application.
pub(crate) fn get_launch_required_version(
    domain: &Domain,
    application_id: u64,
) -> Result<u32, DispatchError> {
    // SAFETY: `application_id` lives on the stack until `.send()` returns.
    let result = unsafe {
        domain
            .dispatch(proto::GET_LAUNCH_REQUIRED_VERSION)
            .in_raw((&raw const application_id).cast::<u8>(), size_of::<u64>())
            .out_size(size_of::<u32>())
            .send()?
    };
    // SAFETY: the response payload is at least `size_of::<u32>()` bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}

/// Upgrades the launch-required version for an application.
pub(crate) fn upgrade_launch_required_version(
    domain: &Domain,
    application_id: u64,
    version: u32,
) -> Result<(), DispatchError> {
    let input = PushVersionIn {
        version,
        application_id,
    };
    // SAFETY: `input` lives on the stack until `.send()` returns.
    unsafe {
        domain
            .dispatch(proto::UPGRADE_LAUNCH_REQUIRED_VERSION)
            .in_raw((&raw const input).cast::<u8>(), size_of::<PushVersionIn>())
            .send()
            .map(|_| ())
    }
}

/// Pushes the launch version for an application.
pub(crate) fn push_launch_version(
    domain: &Domain,
    application_id: u64,
    version: u32,
) -> Result<(), DispatchError> {
    let input = PushVersionIn {
        version,
        application_id,
    };
    // SAFETY: `input` lives on the stack until `.send()` returns.
    unsafe {
        domain
            .dispatch(proto::PUSH_LAUNCH_VERSION)
            .in_raw((&raw const input).cast::<u8>(), size_of::<PushVersionIn>())
            .send()
            .map(|_| ())
    }
}

/// Lists all version list entries into a buffer.
pub(crate) fn list_version_list(
    domain: &Domain,
    buffer: &mut [AvmVersionListEntry],
) -> Result<u32, DispatchError> {
    let result = domain
        .dispatch(proto::LIST_VERSION_LIST)
        .out_size(size_of::<u32>())
        .buffer(
            buffer.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(buffer),
            BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send()?;

    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Lists all required-version entries into a buffer.
pub(crate) fn list_required_version(
    domain: &Domain,
    buffer: &mut [AvmRequiredVersionEntry],
) -> Result<u32, DispatchError> {
    let result = domain
        .dispatch(proto::LIST_REQUIRED_VERSION)
        .out_size(size_of::<u32>())
        .buffer(
            buffer.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(buffer),
            BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send()?;

    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

// VersionListImporter commands

/// Sets the timestamp on the importer.
pub(crate) fn importer_set_timestamp(
    object: &DomainObject<'_>,
    timestamp: u64,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::IMPORTER_SET_TIMESTAMP, timestamp)
}

/// Sets the version list data on the importer.
pub(crate) fn importer_set_data(
    object: &DomainObject<'_>,
    entries: &[AvmVersionListEntry],
) -> Result<(), DispatchError> {
    object
        .dispatch(proto::IMPORTER_SET_DATA)
        .buffer(
            entries.as_ptr().cast::<u8>(),
            core::mem::size_of_val(entries),
            BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send()
        .map(|_| ())
}

/// Flushes the importer, committing the data.
pub(crate) fn importer_flush(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::IMPORTER_FLUSH)
}

/// Error returned by [`get_version_list_importer`].
#[derive(Debug, thiserror::Error)]
pub enum GetVersionListImporterError {
    /// IPC dispatch failed.
    #[error("failed to dispatch GetVersionListImporter")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected sub-object id.
    #[error("GetVersionListImporter response did not include the expected sub-object")]
    MissingObject,
}
