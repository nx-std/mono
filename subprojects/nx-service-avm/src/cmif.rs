//! CMIF protocol operations for the AVM service.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Domain,
    DomainObject,
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::{
        dispatch_in,
        dispatch_no_io,
    },
    proto,
    types::{
        AvmRequiredVersionEntry,
        AvmVersionListEntry,
        GetVersionIn,
        PushVersionIn,
    },
};

/// Gets the highest available version for a title pair.
pub(crate) fn get_highest_available_version(
    domain: &Domain,
    id_1: u64,
    id_2: u64,
) -> Result<u32, DispatchError> {
    let input = GetVersionIn { id_1, id_2 };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::GET_HIGHEST_AVAILABLE_VERSION)
        .in_raw(input.as_bytes())
        .out_size(size_of::<u32>())
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u32>())
}

/// Gets the highest required version for a title pair.
pub(crate) fn get_highest_required_version(
    domain: &Domain,
    id_1: u64,
    id_2: u64,
) -> Result<u32, DispatchError> {
    let input = GetVersionIn { id_1, id_2 };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::GET_HIGHEST_REQUIRED_VERSION)
        .in_raw(input.as_bytes())
        .out_size(size_of::<u32>())
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u32>())
}

/// Gets a single version list entry by application ID.
pub(crate) fn get_version_list_entry(
    domain: &Domain,
    application_id: u64,
) -> Result<AvmVersionListEntry, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::GET_VERSION_LIST_ENTRY)
        .in_raw(application_id.as_bytes())
        .out_size(size_of::<AvmVersionListEntry>())
        .send(&mut ipc_buf)?;
    Ok(*result.value::<AvmVersionListEntry>())
}

/// Gets a version list importer sub-object.
pub(crate) fn get_version_list_importer<'d>(
    domain: &'d Domain,
) -> Result<DomainObject<'d>, GetVersionListImporterError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let mut result = domain
        .dispatch(proto::GET_VERSION_LIST_IMPORTER)
        .out_objects(1)
        .send(&mut ipc_buf)
        .map_err(GetVersionListImporterError::Dispatch)?;

    result
        .take_object(0)
        .ok_or(GetVersionListImporterError::MissingObject)
}

/// Gets the launch-required version for an application.
pub(crate) fn get_launch_required_version(
    domain: &Domain,
    application_id: u64,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::GET_LAUNCH_REQUIRED_VERSION)
        .in_raw(application_id.as_bytes())
        .out_size(size_of::<u32>())
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u32>())
}

/// Upgrades the launch-required version for an application.
pub(crate) fn upgrade_launch_required_version(
    domain: &Domain,
    application_id: u64,
    version: u32,
) -> Result<(), DispatchError> {
    let input = PushVersionIn::new(version, application_id);
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::UPGRADE_LAUNCH_REQUIRED_VERSION)
        .in_raw(input.as_bytes())
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Pushes the launch version for an application.
pub(crate) fn push_launch_version(
    domain: &Domain,
    application_id: u64,
    version: u32,
) -> Result<(), DispatchError> {
    let input = PushVersionIn::new(version, application_id);
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::PUSH_LAUNCH_VERSION)
        .in_raw(input.as_bytes())
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Lists all version list entries into a buffer.
pub(crate) fn list_version_list(
    domain: &Domain,
    buffer: &mut [AvmVersionListEntry],
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::LIST_VERSION_LIST)
        .out_size(size_of::<u32>())
        .out_buffer(buffer.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u32>())
}

/// Lists all required-version entries into a buffer.
pub(crate) fn list_required_version(
    domain: &Domain,
    buffer: &mut [AvmRequiredVersionEntry],
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::LIST_REQUIRED_VERSION)
        .out_size(size_of::<u32>())
        .out_buffer(buffer.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u32>())
}

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
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::IMPORTER_SET_DATA)
        .in_buffer(entries.as_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
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
