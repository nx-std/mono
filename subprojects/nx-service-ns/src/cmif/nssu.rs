//! ns:su + ISystemUpdateControl CMIF commands.

use core::{mem::size_of, ptr};

use nx_sf::service::{BufferAttr, DispatchError, OutHandleAttr, Session};

use super::app_manager::{AsyncCommandError, AsyncOut};
use crate::{
    dispatch::{dispatch_in, dispatch_no_io, dispatch_out},
    proto,
    types::{NcmContentMetaKey, RequestSendReceiveSystemUpdateIn, SystemUpdateProgress},
};

// ---------------------------------------------------------------------------
// ns:su top-level commands
// ---------------------------------------------------------------------------

/// GetBackgroundNetworkUpdateState (cmd 0).
#[inline]
pub(crate) fn get_background_network_update_state(service: &Session) -> Result<u8, DispatchError> {
    dispatch_out(service, proto::NSSU_GET_BACKGROUND_NETWORK_UPDATE_STATE)
}

/// OpenSystemUpdateControl (cmd 1) — returns move handle.
pub(crate) fn open_system_update_control(
    service: &Session,
) -> Result<u32, OpenSystemUpdateControlError> {
    let result = service
        .dispatch(proto::NSSU_OPEN_SYSTEM_UPDATE_CONTROL)
        .send()
        .map_err(OpenSystemUpdateControlError::Dispatch)?;

    let Some(handle) = result.move_handles.first().copied() else {
        return Err(OpenSystemUpdateControlError::MissingHandle);
    };

    Ok(handle)
}

/// NotifyExFatDriverRequired (cmd 2).
#[inline]
pub(crate) fn notify_exfat_driver_required(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::NSSU_NOTIFY_EXFAT_DRIVER_REQUIRED)
}

/// ClearExFatDriverStatusForDebug (cmd 3).
#[inline]
pub(crate) fn clear_exfat_driver_status_for_debug(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::NSSU_CLEAR_EXFAT_DRIVER_STATUS_FOR_DEBUG)
}

/// RequestBackgroundNetworkUpdate (cmd 4).
#[inline]
pub(crate) fn request_background_network_update(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::NSSU_REQUEST_BACKGROUND_NETWORK_UPDATE)
}

/// NotifyBackgroundNetworkUpdate (cmd 5).
#[inline]
pub(crate) fn notify_background_network_update(
    service: &Session,
    key: NcmContentMetaKey,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::NSSU_NOTIFY_BACKGROUND_NETWORK_UPDATE, key)
}

/// NotifyExFatDriverDownloadedForDebug (cmd 6).
#[inline]
pub(crate) fn notify_exfat_driver_downloaded_for_debug(
    service: &Session,
) -> Result<(), DispatchError> {
    dispatch_no_io(
        service,
        proto::NSSU_NOTIFY_EXFAT_DRIVER_DOWNLOADED_FOR_DEBUG,
    )
}

/// GetSystemUpdateNotificationEventForContentDelivery (cmd 9) — returns copy handle.
pub(crate) fn get_system_update_notification_event(
    service: &Session,
) -> Result<u32, AcquireEventError> {
    let result = service
        .dispatch(proto::NSSU_GET_SYSTEM_UPDATE_NOTIFICATION_EVENT_FOR_CONTENT_DELIVERY)
        .out_handle(0, OutHandleAttr::Copy)
        .send()
        .map_err(AcquireEventError::Dispatch)?;

    let Some(handle) = result.copy_handles.first().copied() else {
        return Err(AcquireEventError::MissingHandle);
    };

    Ok(handle)
}

/// NotifySystemUpdateForContentDelivery (cmd 10).
#[inline]
pub(crate) fn notify_system_update_for_content_delivery(
    service: &Session,
) -> Result<(), DispatchError> {
    dispatch_no_io(
        service,
        proto::NSSU_NOTIFY_SYSTEM_UPDATE_FOR_CONTENT_DELIVERY,
    )
}

/// PrepareShutdown (cmd 11).
#[inline]
pub(crate) fn prepare_shutdown(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::NSSU_PREPARE_SHUTDOWN)
}

/// DestroySystemUpdateTask (cmd 16).
#[inline]
pub(crate) fn destroy_system_update_task(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::NSSU_DESTROY_SYSTEM_UPDATE_TASK)
}

/// RequestSendSystemUpdate (cmd 17).
pub(crate) fn request_send_system_update(
    service: &Session,
    input: RequestSendReceiveSystemUpdateIn,
    system_delivery_info: &[u8],
) -> Result<AsyncOut, AsyncCommandError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<RequestSendReceiveSystemUpdateIn>(),
        )
    };
    let result = service
        .dispatch(proto::NSSU_REQUEST_SEND_SYSTEM_UPDATE)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .in_buffer(system_delivery_info, BufferAttr::HIPC_MAP_ALIAS)
        .send()
        .map_err(AsyncCommandError::Dispatch)?;

    super::app_manager::extract_async_out(&result)
}

/// GetSendSystemUpdateProgress (cmd 18).
#[inline]
pub(crate) fn get_send_system_update_progress(
    service: &Session,
) -> Result<SystemUpdateProgress, DispatchError> {
    dispatch_out(service, proto::NSSU_GET_SEND_SYSTEM_UPDATE_PROGRESS)
}

// ---------------------------------------------------------------------------
// ISystemUpdateControl commands (dispatched on the control session)
// ---------------------------------------------------------------------------

/// HasDownloaded (ctrl cmd 0).
#[inline]
pub(crate) fn ctrl_has_downloaded(service: &Session) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(service, proto::NSSU_CTRL_HAS_DOWNLOADED)?;
    Ok(val & 1 != 0)
}

/// RequestCheckLatestUpdate (ctrl cmd 1).
pub(crate) fn ctrl_request_check_latest_update(
    service: &Session,
) -> Result<AsyncOut, AsyncCommandError> {
    dispatch_ctrl_async_no_in(service, proto::NSSU_CTRL_REQUEST_CHECK_LATEST_UPDATE)
}

/// RequestDownloadLatestUpdate (ctrl cmd 2).
pub(crate) fn ctrl_request_download_latest_update(
    service: &Session,
) -> Result<AsyncOut, AsyncCommandError> {
    dispatch_ctrl_async_no_in(service, proto::NSSU_CTRL_REQUEST_DOWNLOAD_LATEST_UPDATE)
}

/// GetDownloadProgress (ctrl cmd 3).
#[inline]
pub(crate) fn ctrl_get_download_progress(
    service: &Session,
) -> Result<SystemUpdateProgress, DispatchError> {
    dispatch_out(service, proto::NSSU_CTRL_GET_DOWNLOAD_PROGRESS)
}

/// ApplyDownloadedUpdate (ctrl cmd 4).
#[inline]
pub(crate) fn ctrl_apply_downloaded_update(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::NSSU_CTRL_APPLY_DOWNLOADED_UPDATE)
}

/// RequestPrepareCardUpdate (ctrl cmd 5).
pub(crate) fn ctrl_request_prepare_card_update(
    service: &Session,
) -> Result<AsyncOut, AsyncCommandError> {
    dispatch_ctrl_async_no_in(service, proto::NSSU_CTRL_REQUEST_PREPARE_CARD_UPDATE)
}

/// GetPrepareCardUpdateProgress (ctrl cmd 6).
#[inline]
pub(crate) fn ctrl_get_prepare_card_update_progress(
    service: &Session,
) -> Result<SystemUpdateProgress, DispatchError> {
    dispatch_out(service, proto::NSSU_CTRL_GET_PREPARE_CARD_UPDATE_PROGRESS)
}

/// HasPreparedCardUpdate (ctrl cmd 7).
#[inline]
pub(crate) fn ctrl_has_prepared_card_update(service: &Session) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(service, proto::NSSU_CTRL_HAS_PREPARED_CARD_UPDATE)?;
    Ok(val & 1 != 0)
}

/// ApplyCardUpdate (ctrl cmd 8).
#[inline]
pub(crate) fn ctrl_apply_card_update(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::NSSU_CTRL_APPLY_CARD_UPDATE)
}

/// GetDownloadedEulaDataSize (ctrl cmd 9).
pub(crate) fn ctrl_get_downloaded_eula_data_size(
    service: &Session,
    path: &[u8],
) -> Result<u64, DispatchError> {
    let result = service
        .dispatch(proto::NSSU_CTRL_GET_DOWNLOADED_EULA_DATA_SIZE)
        .out_size(size_of::<u64>())
        .in_buffer(path, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) })
}

/// GetDownloadedEulaData (ctrl cmd 10).
pub(crate) fn ctrl_get_downloaded_eula_data(
    service: &Session,
    path: &[u8],
    out: &mut [u8],
) -> Result<u64, DispatchError> {
    let result = service
        .dispatch(proto::NSSU_CTRL_GET_DOWNLOADED_EULA_DATA)
        .out_size(size_of::<u64>())
        .in_buffer(path, BufferAttr::HIPC_MAP_ALIAS)
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) })
}

/// SetupCardUpdate (ctrl cmd 11).
pub(crate) fn ctrl_setup_card_update(
    service: &Session,
    tmem_size: u64,
    tmem_handle: u32,
) -> Result<(), DispatchError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const tmem_size).cast::<u8>(), size_of::<u64>())
    };
    service
        .dispatch(proto::NSSU_CTRL_SETUP_CARD_UPDATE)
        .in_raw(in_bytes)
        .in_handle(tmem_handle)
        .send()
        .map(|_| ())
}

/// GetPreparedCardUpdateEulaDataSize (ctrl cmd 12).
pub(crate) fn ctrl_get_prepared_card_update_eula_data_size(
    service: &Session,
    path: &[u8],
) -> Result<u64, DispatchError> {
    let result = service
        .dispatch(proto::NSSU_CTRL_GET_PREPARED_CARD_UPDATE_EULA_DATA_SIZE)
        .out_size(size_of::<u64>())
        .in_buffer(path, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) })
}

/// GetPreparedCardUpdateEulaData (ctrl cmd 13).
pub(crate) fn ctrl_get_prepared_card_update_eula_data(
    service: &Session,
    path: &[u8],
    out: &mut [u8],
) -> Result<u64, DispatchError> {
    let result = service
        .dispatch(proto::NSSU_CTRL_GET_PREPARED_CARD_UPDATE_EULA_DATA)
        .out_size(size_of::<u64>())
        .in_buffer(path, BufferAttr::HIPC_MAP_ALIAS)
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) })
}

/// SetupCardUpdateViaSystemUpdater (ctrl cmd 14).
pub(crate) fn ctrl_setup_card_update_via_system_updater(
    service: &Session,
    tmem_size: u64,
    tmem_handle: u32,
) -> Result<(), DispatchError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const tmem_size).cast::<u8>(), size_of::<u64>())
    };
    service
        .dispatch(proto::NSSU_CTRL_SETUP_CARD_UPDATE_VIA_SYSTEM_UPDATER)
        .in_raw(in_bytes)
        .in_handle(tmem_handle)
        .send()
        .map(|_| ())
}

/// HasReceived (ctrl cmd 15).
#[inline]
pub(crate) fn ctrl_has_received(service: &Session) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(service, proto::NSSU_CTRL_HAS_RECEIVED)?;
    Ok(val & 1 != 0)
}

/// RequestReceiveSystemUpdate (ctrl cmd 16).
pub(crate) fn ctrl_request_receive_system_update(
    service: &Session,
    input: RequestSendReceiveSystemUpdateIn,
) -> Result<AsyncOut, AsyncCommandError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<RequestSendReceiveSystemUpdateIn>(),
        )
    };
    let result = service
        .dispatch(proto::NSSU_CTRL_REQUEST_RECEIVE_SYSTEM_UPDATE)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .send()
        .map_err(AsyncCommandError::Dispatch)?;

    super::app_manager::extract_async_out(&result)
}

/// GetReceiveProgress (ctrl cmd 17).
#[inline]
pub(crate) fn ctrl_get_receive_progress(
    service: &Session,
) -> Result<SystemUpdateProgress, DispatchError> {
    dispatch_out(service, proto::NSSU_CTRL_GET_RECEIVE_PROGRESS)
}

/// ApplyReceivedUpdate (ctrl cmd 18).
#[inline]
pub(crate) fn ctrl_apply_received_update(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::NSSU_CTRL_APPLY_RECEIVED_UPDATE)
}

/// GetReceivedEulaDataSize (ctrl cmd 19).
pub(crate) fn ctrl_get_received_eula_data_size(
    service: &Session,
    path: &[u8],
) -> Result<u64, DispatchError> {
    let result = service
        .dispatch(proto::NSSU_CTRL_GET_RECEIVED_EULA_DATA_SIZE)
        .out_size(size_of::<u64>())
        .in_buffer(path, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) })
}

/// GetReceivedEulaData (ctrl cmd 20).
pub(crate) fn ctrl_get_received_eula_data(
    service: &Session,
    path: &[u8],
    out: &mut [u8],
) -> Result<u64, DispatchError> {
    let result = service
        .dispatch(proto::NSSU_CTRL_GET_RECEIVED_EULA_DATA)
        .out_size(size_of::<u64>())
        .in_buffer(path, BufferAttr::HIPC_MAP_ALIAS)
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) })
}

/// SetupToReceiveSystemUpdate (ctrl cmd 21).
#[inline]
pub(crate) fn ctrl_setup_to_receive_system_update(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::NSSU_CTRL_SETUP_TO_RECEIVE_SYSTEM_UPDATE)
}

/// RequestCheckLatestUpdateIncludesRebootlessUpdate (ctrl cmd 22).
pub(crate) fn ctrl_request_check_latest_update_includes_rebootless_update(
    service: &Session,
) -> Result<AsyncOut, AsyncCommandError> {
    dispatch_ctrl_async_no_in(
        service,
        proto::NSSU_CTRL_REQUEST_CHECK_LATEST_UPDATE_INCLUDES_REBOOTLESS_UPDATE,
    )
}

// ---------------------------------------------------------------------------
// Shared dispatch helpers
// ---------------------------------------------------------------------------

/// Dispatches an async control command with no input.
fn dispatch_ctrl_async_no_in(
    service: &Session,
    cmd_id: u32,
) -> Result<AsyncOut, AsyncCommandError> {
    let result = service
        .dispatch(cmd_id)
        .out_handle(0, OutHandleAttr::Copy)
        .send()
        .map_err(AsyncCommandError::Dispatch)?;

    super::app_manager::extract_async_out(&result)
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned by [`open_system_update_control`].
#[derive(Debug, thiserror::Error)]
pub enum OpenSystemUpdateControlError {
    #[error("failed to dispatch OpenSystemUpdateControl")]
    Dispatch(#[source] DispatchError),
    #[error("OpenSystemUpdateControl response did not include expected move handle")]
    MissingHandle,
}

/// Error returned by event acquisition commands.
#[derive(Debug, thiserror::Error)]
pub enum AcquireEventError {
    #[error("failed to dispatch event acquisition")]
    Dispatch(#[source] DispatchError),
    #[error("event acquisition response did not include expected copy handle")]
    MissingHandle,
}
