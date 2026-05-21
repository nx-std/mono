//! CMIF protocol operations for the news service.

use core::{mem::size_of, ptr};

use nx_sf::service::{BufferAttr, DispatchError, OutHandleAttr, Session};

use crate::{
    dispatch::{dispatch_no_io, dispatch_out},
    proto,
    types::{NewsRecord, NewsRecordV1, SavedataUsageOut},
};

// ---------------------------------------------------------------------------
// Creator commands (used on the creator session, 2.0.0+)
// ---------------------------------------------------------------------------

/// Creates the news service sub-object from the creator session.
///
/// Returns the raw move handle for the new session.
pub(crate) fn create_news_service(service: &Session) -> Result<u32, CreateSubObjectError> {
    create_sub_object(service, proto::CREATE_NEWS_SERVICE)
}

/// Creates a sub-object and returns its raw move handle.
pub(crate) fn create_sub_object(
    service: &Session,
    cmd_id: u32,
) -> Result<u32, CreateSubObjectError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(cmd_id)
        .send(&mut ipc_buf)
        .map_err(CreateSubObjectError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(CreateSubObjectError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

// ---------------------------------------------------------------------------
// INewsService commands
// ---------------------------------------------------------------------------

/// Posts local news (HipcMapAlias input buffer).
pub(crate) fn post_local_news(service: &Session, news: &[u8]) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::POST_LOCAL_NEWS)
        .in_buffer(news, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Sets passphrase for a program (HipcPointer input buffer).
pub(crate) fn set_passphrase(
    service: &Session,
    program_id: u64,
    passphrase: &[u8],
) -> Result<(), DispatchError> {
    // SAFETY: `program_id` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const program_id).cast::<u8>(), size_of::<u64>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::SET_PASSPHRASE)
        .in_raw(in_bytes)
        .in_buffer(passphrase, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Gets subscription status for a filter string.
pub(crate) fn get_subscription_status(
    service: &Session,
    filter: &[u8],
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::GET_SUBSCRIPTION_STATUS)
        .out_size(size_of::<u32>())
        .in_buffer(filter, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<u32>().
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}

/// Gets the topic list (3.0.0+).
pub(crate) fn get_topic_list(
    service: &Session,
    channel: u32,
    out_buf: &mut [u8],
) -> Result<u32, DispatchError> {
    // SAFETY: `channel` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const channel).cast::<u8>(), size_of::<u32>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::GET_TOPIC_LIST)
        .in_raw(in_bytes)
        .out_size(size_of::<u32>())
        .out_buffer(out_buf, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<u32>().
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}

/// Gets save data usage (6.0.0+).
pub(crate) fn get_savedata_usage(service: &Session) -> Result<SavedataUsageOut, DispatchError> {
    dispatch_out(service, proto::GET_SAVEDATA_USAGE)
}

/// Checks if a system update is required.
pub(crate) fn is_system_update_required(service: &Session) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(service, proto::IS_SYSTEM_UPDATE_REQUIRED)?;
    Ok(val & 1 != 0)
}

/// Gets database version (10.0.0+).
pub(crate) fn get_database_version(service: &Session) -> Result<u32, DispatchError> {
    dispatch_out(service, proto::GET_DATABASE_VERSION)
}

/// Requests immediate reception with a filter.
pub(crate) fn request_immediate_reception(
    service: &Session,
    filter: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::REQUEST_IMMEDIATE_RECEPTION)
        .in_buffer(filter, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Sets subscription status for a filter.
pub(crate) fn set_subscription_status(
    service: &Session,
    status: u32,
    filter: &[u8],
) -> Result<(), DispatchError> {
    // SAFETY: `status` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const status).cast::<u8>(), size_of::<u32>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::SET_SUBSCRIPTION_STATUS)
        .in_raw(in_bytes)
        .in_buffer(filter, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Clears all news storage.
pub(crate) fn clear_storage(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::CLEAR_STORAGE)
}

/// Clears all subscription statuses.
pub(crate) fn clear_subscription_status_all(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::CLEAR_SUBSCRIPTION_STATUS_ALL)
}

/// Gets the news database dump (HipcMapAlias output buffer).
pub(crate) fn get_news_database_dump(
    service: &Session,
    buffer: &mut [u8],
) -> Result<u64, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::GET_NEWS_DATABASE_DUMP)
        .out_size(size_of::<u64>())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<u64>().
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) })
}

// ---------------------------------------------------------------------------
// INewsNewlyArrivedEventHolder / INewsOverwriteEventHolder commands
// ---------------------------------------------------------------------------

/// Gets the event handle from an event holder sub-object (cmd 0).
pub(crate) fn event_holder_get(service: &Session) -> Result<u32, EventHolderGetError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::EVENT_HOLDER_GET)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(EventHolderGetError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(EventHolderGetError::MissingHandle);
    }
    Ok(result.copy_handles[0])
}

// ---------------------------------------------------------------------------
// INewsDataService commands
// ---------------------------------------------------------------------------

/// Opens news data by file name (HipcPointer input).
pub(crate) fn data_open(service: &Session, file_name: &[u8]) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::DATA_OPEN)
        .in_buffer(file_name, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Opens news data with a V1 record (pre-6.0.0).
pub(crate) fn data_open_with_record_v1(
    service: &Session,
    record: &NewsRecordV1,
) -> Result<(), DispatchError> {
    // SAFETY: `record` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const *record).cast::<u8>(), size_of::<NewsRecordV1>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::DATA_OPEN_WITH_RECORD_V1)
        .in_raw(in_bytes)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Reads data from the opened news data (auto-select buffer).
pub(crate) fn data_read(
    service: &Session,
    offset: u64,
    out_buf: &mut [u8],
) -> Result<u64, DispatchError> {
    // SAFETY: `offset` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const offset).cast::<u8>(), size_of::<u64>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::DATA_READ)
        .in_raw(in_bytes)
        .out_size(size_of::<u64>())
        .out_buffer(out_buf, BufferAttr::HIPC_AUTO_SELECT)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<u64>().
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) })
}

/// Gets the size of the opened news data.
pub(crate) fn data_get_size(service: &Session) -> Result<u64, DispatchError> {
    dispatch_out(service, proto::DATA_GET_SIZE)
}

/// Opens news data with a current record (6.0.0+).
pub(crate) fn data_open_with_record(
    service: &Session,
    record: &NewsRecord,
) -> Result<(), DispatchError> {
    // SAFETY: `record` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const *record).cast::<u8>(), size_of::<NewsRecord>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::DATA_OPEN_WITH_RECORD)
        .in_raw(in_bytes)
        .send(&mut ipc_buf)
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// INewsDatabaseService commands
// ---------------------------------------------------------------------------

/// Gets a list of V1 records (pre-6.0.0).
pub(crate) fn database_get_list_v1(
    service: &Session,
    offset: u32,
    out_buf: &mut [u8],
    where_clause: &[u8],
    order_clause: &[u8],
) -> Result<u32, DispatchError> {
    // SAFETY: `offset` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const offset).cast::<u8>(), size_of::<u32>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::DATABASE_GET_LIST_V1)
        .in_raw(in_bytes)
        .out_size(size_of::<u32>())
        .out_buffer(out_buf, BufferAttr::HIPC_AUTO_SELECT)
        .in_buffer(where_clause, BufferAttr::HIPC_POINTER)
        .in_buffer(order_clause, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<u32>().
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}

/// Counts records matching a filter.
pub(crate) fn database_count(service: &Session, filter: &[u8]) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::DATABASE_COUNT)
        .out_size(size_of::<u32>())
        .in_buffer(filter, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<u32>().
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}

/// Gets a list of current records (6.0.0+).
pub(crate) fn database_get_list(
    service: &Session,
    offset: u32,
    out_buf: &mut [u8],
    where_clause: &[u8],
    order_clause: &[u8],
) -> Result<u32, DispatchError> {
    // SAFETY: `offset` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const offset).cast::<u8>(), size_of::<u32>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::DATABASE_GET_LIST)
        .in_raw(in_bytes)
        .out_size(size_of::<u32>())
        .out_buffer(out_buf, BufferAttr::HIPC_AUTO_SELECT)
        .in_buffer(where_clause, BufferAttr::HIPC_POINTER)
        .in_buffer(order_clause, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<u32>().
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned when creating a sub-object from the creator service.
#[derive(Debug, thiserror::Error)]
pub enum CreateSubObjectError {
    /// IPC dispatch failed.
    #[error("failed to dispatch sub-object creation command")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected move handle.
    #[error("sub-object creation response did not include expected move handle")]
    MissingHandle,
}

/// Error returned by [`event_holder_get`].
#[derive(Debug, thiserror::Error)]
pub enum EventHolderGetError {
    /// IPC dispatch failed.
    #[error("failed to dispatch event holder Get")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected event copy handle.
    #[error("event holder Get response did not include expected event handle")]
    MissingHandle,
}
