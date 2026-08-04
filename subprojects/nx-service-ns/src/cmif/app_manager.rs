//! IApplicationManagerInterface CMIF commands.

use core::{
    mem::size_of,
    ptr,
};

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    OutHandleAttr,
    Session,
};

use crate::{
    dispatch::{
        dispatch_in,
        dispatch_in_out,
        dispatch_no_io,
        dispatch_out,
    },
    proto,
    types::{
        AccountUid,
        CleanupUnavailableAddOnContentsIn,
        ContentMetaStatusIn,
        DeleteSaveDataIn,
        DeleteUserSystemSaveDataIn,
        EstimateSizeToMoveIn,
        GameCardRegistrationGoldPointIn,
        GetApplicationDeliveryInfoIn,
        GetApplicationRightsOnClientIn,
        IsEntityMovableIn,
        IsUpdateRequestedOut,
        ListApplicationTitleIn,
        ListNotCommittedContentMetaIn,
        RegisterGameCardIn,
        RequestReceiveApplicationIn,
        RequestSendApplicationIn,
        SetTerminateResultIn,
        StorageIdS64Out,
        StorageSizesOut,
        SystemUpdateProgress,
        VerifyApplicationDeprecatedIn,
        VerifyApplicationIn,
    },
};

// ---------------------------------------------------------------------------
// No I/O commands (dispatch_no_io)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn delete_redundant_application_entity(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::APPMGR_DELETE_REDUNDANT_APPLICATION_ENTITY)
}

#[inline]
pub(crate) fn cleanup_sd_card(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::APPMGR_CLEANUP_SD_CARD)
}

#[inline]
pub(crate) fn check_sd_card_mount_status(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::APPMGR_CHECK_SD_CARD_MOUNT_STATUS)
}

#[inline]
pub(crate) fn get_last_sd_card_mount_unexpected_result(
    service: &Session,
) -> Result<(), DispatchError> {
    dispatch_no_io(
        service,
        proto::APPMGR_GET_LAST_SD_CARD_MOUNT_UNEXPECTED_RESULT,
    )
}

#[inline]
pub(crate) fn resume_all(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::APPMGR_RESUME_ALL)
}

#[inline]
pub(crate) fn ensure_game_card_access(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::APPMGR_ENSURE_GAME_CARD_ACCESS)
}

#[inline]
pub(crate) fn get_last_game_card_mount_failure_result(
    service: &Session,
) -> Result<(), DispatchError> {
    dispatch_no_io(
        service,
        proto::APPMGR_GET_LAST_GAME_CARD_MOUNT_FAILURE_RESULT,
    )
}

#[inline]
pub(crate) fn format_sd_card(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::APPMGR_FORMAT_SD_CARD)
}

#[inline]
pub(crate) fn clear_task_status_list(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::APPMGR_CLEAR_TASK_STATUS_LIST)
}

#[inline]
pub(crate) fn request_download_task_list(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::APPMGR_REQUEST_DOWNLOAD_TASK_LIST)
}

#[inline]
pub(crate) fn try_commit_current_application_download_task(
    service: &Session,
) -> Result<(), DispatchError> {
    dispatch_no_io(
        service,
        proto::APPMGR_TRY_COMMIT_CURRENT_APPLICATION_DOWNLOAD_TASK,
    )
}

#[inline]
pub(crate) fn enable_auto_commit(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::APPMGR_ENABLE_AUTO_COMMIT)
}

#[inline]
pub(crate) fn disable_auto_commit(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::APPMGR_DISABLE_AUTO_COMMIT)
}

#[inline]
pub(crate) fn trigger_dynamic_commit_event(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::APPMGR_TRIGGER_DYNAMIC_COMMIT_EVENT)
}

// ---------------------------------------------------------------------------
// u64 input, no output (dispatch_in)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn delete_application_entity(
    service: &Session,
    application_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPMGR_DELETE_APPLICATION_ENTITY,
        application_id,
    )
}

#[inline]
pub(crate) fn delete_application_completely(
    service: &Session,
    application_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPMGR_DELETE_APPLICATION_COMPLETELY,
        application_id,
    )
}

#[inline]
pub(crate) fn cancel_application_download(
    service: &Session,
    application_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPMGR_CANCEL_APPLICATION_DOWNLOAD,
        application_id,
    )
}

#[inline]
pub(crate) fn resume_application_download(
    service: &Session,
    application_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPMGR_RESUME_APPLICATION_DOWNLOAD,
        application_id,
    )
}

#[inline]
pub(crate) fn check_application_launch_version(
    service: &Session,
    application_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPMGR_CHECK_APPLICATION_LAUNCH_VERSION,
        application_id,
    )
}

#[inline]
pub(crate) fn disable_application_auto_delete(
    service: &Session,
    application_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPMGR_DISABLE_APPLICATION_AUTO_DELETE,
        application_id,
    )
}

#[inline]
pub(crate) fn enable_application_auto_delete(
    service: &Session,
    application_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPMGR_ENABLE_APPLICATION_AUTO_DELETE,
        application_id,
    )
}

#[inline]
pub(crate) fn clear_application_terminate_result(
    service: &Session,
    application_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPMGR_CLEAR_APPLICATION_TERMINATE_RESULT,
        application_id,
    )
}

#[inline]
pub(crate) fn cancel_application_apply_delta(
    service: &Session,
    application_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPMGR_CANCEL_APPLICATION_APPLY_DELTA,
        application_id,
    )
}

#[inline]
pub(crate) fn resume_application_apply_delta(
    service: &Session,
    application_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPMGR_RESUME_APPLICATION_APPLY_DELTA,
        application_id,
    )
}

#[inline]
pub(crate) fn touch_application(
    service: &Session,
    application_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::APPMGR_TOUCH_APPLICATION, application_id)
}

#[inline]
pub(crate) fn withdraw_application_update_request(
    service: &Session,
    application_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPMGR_WITHDRAW_APPLICATION_UPDATE_REQUEST,
        application_id,
    )
}

#[inline]
pub(crate) fn commit_receive_application(
    service: &Session,
    application_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPMGR_COMMIT_RECEIVE_APPLICATION,
        application_id,
    )
}

// ---------------------------------------------------------------------------
// u64 input, StorageIdS64Out output (dispatch_in_out)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn calculate_application_download_required_size(
    service: &Session,
    application_id: u64,
) -> Result<StorageIdS64Out, DispatchError> {
    dispatch_in_out(
        service,
        proto::APPMGR_CALCULATE_APPLICATION_DOWNLOAD_REQUIRED_SIZE,
        application_id,
    )
}

#[inline]
pub(crate) fn calculate_application_apply_delta_required_size(
    service: &Session,
    application_id: u64,
) -> Result<StorageIdS64Out, DispatchError> {
    dispatch_in_out(
        service,
        proto::APPMGR_CALCULATE_APPLICATION_APPLY_DELTA_REQUIRED_SIZE,
        application_id,
    )
}

// ---------------------------------------------------------------------------
// u64 input, u64 output
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn get_total_space_size(
    service: &Session,
    storage_id: u64,
) -> Result<u64, DispatchError> {
    dispatch_in_out(service, proto::APPMGR_GET_TOTAL_SPACE_SIZE, storage_id)
}

#[inline]
pub(crate) fn get_free_space_size(
    service: &Session,
    storage_id: u64,
) -> Result<u64, DispatchError> {
    dispatch_in_out(service, proto::APPMGR_GET_FREE_SPACE_SIZE, storage_id)
}

// ---------------------------------------------------------------------------
// u64 input, bool output
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn is_any_application_entity_installed(
    service: &Session,
    application_id: u64,
) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_in_out(
        service,
        proto::APPMGR_IS_ANY_APPLICATION_ENTITY_INSTALLED,
        application_id,
    )?;
    Ok(val & 1 != 0)
}

#[inline]
pub(crate) fn is_game_card_inserted(
    service: &Session,
    application_id: u64,
) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_in_out(service, proto::APPMGR_IS_GAME_CARD_INSERTED, application_id)?;
    Ok(val & 1 != 0)
}

// ---------------------------------------------------------------------------
// No input, bool output
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn needs_system_update_to_format_sd_card(
    service: &Session,
) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(service, proto::APPMGR_NEEDS_SYSTEM_UPDATE_TO_FORMAT_SD_CARD)?;
    Ok(val & 1 != 0)
}

#[inline]
pub(crate) fn is_any_application_running(service: &Session) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(service, proto::APPMGR_IS_ANY_APPLICATION_RUNNING)?;
    Ok(val & 1 != 0)
}

// ---------------------------------------------------------------------------
// Event commands (copy handle out)
// ---------------------------------------------------------------------------

pub(crate) fn get_application_record_update_system_event(
    service: &Session,
) -> Result<u32, AcquireEventError> {
    acquire_event(
        service,
        proto::APPMGR_GET_APPLICATION_RECORD_UPDATE_SYSTEM_EVENT,
    )
}

pub(crate) fn get_sd_card_mount_status_changed_event(
    service: &Session,
) -> Result<u32, AcquireEventError> {
    acquire_event(
        service,
        proto::APPMGR_GET_SD_CARD_MOUNT_STATUS_CHANGED_EVENT,
    )
}

pub(crate) fn get_game_card_update_detection_event(
    service: &Session,
) -> Result<u32, AcquireEventError> {
    acquire_event(service, proto::APPMGR_GET_GAME_CARD_UPDATE_DETECTION_EVENT)
}

pub(crate) fn get_game_card_mount_failure_event(
    service: &Session,
) -> Result<u32, AcquireEventError> {
    acquire_event(service, proto::APPMGR_GET_GAME_CARD_MOUNT_FAILURE_EVENT)
}

// ---------------------------------------------------------------------------
// Compound input commands
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn is_application_entity_movable(
    service: &Session,
    input: IsEntityMovableIn,
) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_in_out(service, proto::APPMGR_IS_APPLICATION_ENTITY_MOVABLE, input)?;
    Ok(val & 1 != 0)
}

#[inline]
pub(crate) fn move_application_entity(
    service: &Session,
    input: IsEntityMovableIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::APPMGR_MOVE_APPLICATION_ENTITY, input)
}

#[inline]
pub(crate) fn set_application_terminate_result(
    service: &Session,
    input: SetTerminateResultIn,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPMGR_SET_APPLICATION_TERMINATE_RESULT,
        input,
    )
}

#[inline]
pub(crate) fn delete_user_system_save_data(
    service: &Session,
    input: DeleteUserSystemSaveDataIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::APPMGR_DELETE_USER_SYSTEM_SAVE_DATA, input)
}

#[inline]
pub(crate) fn delete_save_data(
    service: &Session,
    input: DeleteSaveDataIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::APPMGR_DELETE_SAVE_DATA, input)
}

#[inline]
pub(crate) fn unregister_network_service_account(
    service: &Session,
    uid: AccountUid,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPMGR_UNREGISTER_NETWORK_SERVICE_ACCOUNT,
        uid,
    )
}

#[inline]
pub(crate) fn unregister_network_service_account_with_user_save_data_deletion(
    service: &Session,
    uid: AccountUid,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPMGR_UNREGISTER_NETWORK_SERVICE_ACCOUNT_WITH_DELETION,
        uid,
    )
}

#[inline]
pub(crate) fn cleanup_unavailable_addon_contents(
    service: &Session,
    input: CleanupUnavailableAddOnContentsIn,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPMGR_CLEANUP_UNAVAILABLE_ADDON_CONTENTS,
        input,
    )
}

#[inline]
pub(crate) fn get_application_terminate_result(
    service: &Session,
    application_id: u64,
) -> Result<u32, DispatchError> {
    dispatch_in_out(
        service,
        proto::APPMGR_GET_APPLICATION_TERMINATE_RESULT,
        application_id,
    )
}

#[inline]
pub(crate) fn get_storage_size(
    service: &Session,
    storage_id: u8,
) -> Result<StorageSizesOut, DispatchError> {
    dispatch_in_out(service, proto::APPMGR_GET_STORAGE_SIZE, storage_id)
}

#[inline]
pub(crate) fn is_application_update_requested(
    service: &Session,
    application_id: u64,
) -> Result<IsUpdateRequestedOut, DispatchError> {
    dispatch_in_out(
        service,
        proto::APPMGR_IS_APPLICATION_UPDATE_REQUESTED,
        application_id,
    )
}

#[inline]
pub(crate) fn count_application_content_meta(
    service: &Session,
    application_id: u64,
) -> Result<i32, DispatchError> {
    dispatch_in_out(
        service,
        proto::APPMGR_COUNT_APPLICATION_CONTENT_META,
        application_id,
    )
}

#[inline]
pub(crate) fn get_receive_application_progress(
    service: &Session,
    application_id: u64,
) -> Result<SystemUpdateProgress, DispatchError> {
    dispatch_in_out(
        service,
        proto::APPMGR_GET_RECEIVE_APPLICATION_PROGRESS,
        application_id,
    )
}

#[inline]
pub(crate) fn get_send_application_progress(
    service: &Session,
    application_id: u64,
) -> Result<SystemUpdateProgress, DispatchError> {
    dispatch_in_out(
        service,
        proto::APPMGR_GET_SEND_APPLICATION_PROGRESS,
        application_id,
    )
}

// ---------------------------------------------------------------------------
// Buffer commands
// ---------------------------------------------------------------------------

pub(crate) fn list_application_record(
    service: &Session,
    records: &mut [u8],
    entry_offset: i32,
) -> Result<i32, DispatchError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const entry_offset).cast::<u8>(), size_of::<i32>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_LIST_APPLICATION_RECORD)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(records, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

pub(crate) fn get_application_view_deprecated(
    service: &Session,
    views: &mut [u8],
    app_ids: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::APPMGR_GET_APPLICATION_VIEW_DEPRECATED)
        .out_buffer(views, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(app_ids, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn get_application_view(
    service: &Session,
    views: &mut [u8],
    app_ids: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::APPMGR_GET_APPLICATION_VIEW)
        .out_buffer(views, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(app_ids, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn get_application_view_with_promotion_info(
    service: &Session,
    views: &mut [u8],
    app_ids: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::APPMGR_GET_APPLICATION_VIEW_WITH_PROMOTION_INFO)
        .out_buffer(views, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(app_ids, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn get_application_view_download_error_context(
    service: &Session,
    application_id: u64,
    out: &mut [u8],
) -> Result<(), DispatchError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const application_id).cast::<u8>(), size_of::<u64>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::APPMGR_GET_APPLICATION_VIEW_DOWNLOAD_ERROR_CONTEXT)
        .in_raw(in_bytes)
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn calculate_application_occupied_size(
    service: &Session,
    application_id: u64,
    out: &mut [u8],
) -> Result<(), DispatchError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const application_id).cast::<u8>(), size_of::<u64>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::APPMGR_CALCULATE_APPLICATION_OCCUPIED_SIZE)
        .in_raw(in_bytes)
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn list_application_content_meta_status(
    service: &Session,
    input: ContentMetaStatusIn,
    out: &mut [u8],
) -> Result<i32, DispatchError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<ContentMetaStatusIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_LIST_APPLICATION_CONTENT_META_STATUS)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

pub(crate) fn list_download_task_status(
    service: &Session,
    out: &mut [u8],
) -> Result<i32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_LIST_DOWNLOAD_TASK_STATUS)
        .out_size(size_of::<i32>())
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

pub(crate) fn list_application_id_on_game_card(
    service: &Session,
    out: &mut [u8],
) -> Result<i32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_LIST_APPLICATION_ID_ON_GAME_CARD)
        .out_size(size_of::<i32>())
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

pub(crate) fn get_system_delivery_info(
    service: &Session,
    out: &mut [u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::APPMGR_GET_SYSTEM_DELIVERY_INFO)
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn verify_delivery_protocol_version(
    service: &Session,
    info: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::APPMGR_VERIFY_DELIVERY_PROTOCOL_VERSION)
        .in_buffer(info, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn get_application_delivery_info(
    service: &Session,
    input: GetApplicationDeliveryInfoIn,
    out: &mut [u8],
) -> Result<i32, DispatchError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<GetApplicationDeliveryInfoIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_GET_APPLICATION_DELIVERY_INFO)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

pub(crate) fn has_all_contents_to_deliver(
    service: &Session,
    delivery_info: &[u8],
) -> Result<bool, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_HAS_ALL_CONTENTS_TO_DELIVER)
        .out_size(size_of::<u8>())
        .in_buffer(delivery_info, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    let val: u8 = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    Ok(val & 1 != 0)
}

pub(crate) fn compare_application_delivery_info(
    service: &Session,
    info_a: &[u8],
    info_b: &[u8],
) -> Result<i32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_COMPARE_APPLICATION_DELIVERY_INFO)
        .out_size(size_of::<i32>())
        .in_buffer(info_a, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(info_b, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

pub(crate) fn can_deliver_application(
    service: &Session,
    info_a: &[u8],
    info_b: &[u8],
) -> Result<bool, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_CAN_DELIVER_APPLICATION)
        .out_size(size_of::<u8>())
        .in_buffer(info_a, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(info_b, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    let val: u8 = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    Ok(val & 1 != 0)
}

pub(crate) fn list_content_meta_key_to_deliver_application(
    service: &Session,
    meta_index: i32,
    out: &mut [u8],
    delivery_info: &[u8],
) -> Result<i32, DispatchError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const meta_index).cast::<u8>(), size_of::<i32>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_LIST_CONTENT_META_KEY_TO_DELIVER_APPLICATION)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(delivery_info, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

pub(crate) fn needs_system_update_to_deliver_application(
    service: &Session,
    system_info: &[u8],
    app_info: &[u8],
) -> Result<bool, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_NEEDS_SYSTEM_UPDATE_TO_DELIVER_APPLICATION)
        .out_size(size_of::<u8>())
        .in_buffer(system_info, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(app_info, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    let val: u8 = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    Ok(val & 1 != 0)
}

pub(crate) fn estimate_required_size(
    service: &Session,
    content_meta_keys: &[u8],
) -> Result<i64, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_ESTIMATE_REQUIRED_SIZE)
        .out_size(size_of::<i64>())
        .in_buffer(content_meta_keys, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i64>()) })
}

pub(crate) fn request_receive_application(
    service: &Session,
    input: RequestReceiveApplicationIn,
    content_meta_keys: &[u8],
) -> Result<AsyncOut, AsyncCommandError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<RequestReceiveApplicationIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_REQUEST_RECEIVE_APPLICATION)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .in_buffer(content_meta_keys, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map_err(AsyncCommandError::Dispatch)?;

    let Some(session_handle) = result.move_handles.first().copied() else {
        return Err(AsyncCommandError::MissingSessionHandle);
    };
    let Some(event_handle) = result.copy_handles.first().copied() else {
        return Err(AsyncCommandError::MissingEventHandle);
    };

    Ok(AsyncOut {
        session_handle,
        event_handle,
    })
}

pub(crate) fn request_send_application(
    service: &Session,
    input: RequestSendApplicationIn,
    content_meta_keys: &[u8],
) -> Result<AsyncOut, AsyncCommandError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<RequestSendApplicationIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_REQUEST_SEND_APPLICATION)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .in_buffer(content_meta_keys, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map_err(AsyncCommandError::Dispatch)?;

    let Some(session_handle) = result.move_handles.first().copied() else {
        return Err(AsyncCommandError::MissingSessionHandle);
    };
    let Some(event_handle) = result.copy_handles.first().copied() else {
        return Err(AsyncCommandError::MissingEventHandle);
    };

    Ok(AsyncOut {
        session_handle,
        event_handle,
    })
}

pub(crate) fn compare_system_delivery_info(
    service: &Session,
    info_a: &[u8],
    info_b: &[u8],
) -> Result<i32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_COMPARE_SYSTEM_DELIVERY_INFO)
        .out_size(size_of::<i32>())
        .in_buffer(info_a, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(info_b, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

pub(crate) fn list_not_committed_content_meta(
    service: &Session,
    input: ListNotCommittedContentMetaIn,
    out: &mut [u8],
) -> Result<i32, DispatchError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<ListNotCommittedContentMetaIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_LIST_NOT_COMMITTED_CONTENT_META)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

pub(crate) fn get_application_delivery_info_hash(
    service: &Session,
    delivery_info: &[u8],
) -> Result<[u8; 0x20], DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_GET_APPLICATION_DELIVERY_INFO_HASH)
        .out_size(0x20)
        .in_buffer(delivery_info, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    let mut hash = [0u8; 0x20];
    hash.copy_from_slice(&result.data[..0x20]);
    Ok(hash)
}

pub(crate) fn get_application_rights_on_client(
    service: &Session,
    input: GetApplicationRightsOnClientIn,
    out: &mut [u8],
) -> Result<i32, DispatchError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<GetApplicationRightsOnClientIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_GET_APPLICATION_RIGHTS_ON_CLIENT)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

pub(crate) fn get_promotion_info(
    service: &Session,
    out: &mut [u8],
    app_id: &[u8],
    uid: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::APPMGR_GET_PROMOTION_INFO)
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(app_id, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(uid, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn select_latest_system_delivery_info(
    service: &Session,
    info_a: &[u8],
    info_b: &[u8],
    info_c: &[u8],
) -> Result<i32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_SELECT_LATEST_SYSTEM_DELIVERY_INFO)
        .out_size(size_of::<i32>())
        .in_buffer(info_a, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(info_b, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(info_c, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

pub(crate) fn estimate_size_to_move(
    service: &Session,
    input: EstimateSizeToMoveIn,
    content_meta_keys: &[u8],
) -> Result<i64, DispatchError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<EstimateSizeToMoveIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_ESTIMATE_SIZE_TO_MOVE)
        .in_raw(in_bytes)
        .out_size(size_of::<i64>())
        .in_buffer(content_meta_keys, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i64>()) })
}

// ---------------------------------------------------------------------------
// Sub-object creation commands (return move handle)
// ---------------------------------------------------------------------------

pub(crate) fn get_request_server_stopper(service: &Session) -> Result<u32, GetSubObjectError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_GET_REQUEST_SERVER_STOPPER)
        .send(&mut ipc_buf)
        .map_err(GetSubObjectError::Dispatch)?;

    let Some(handle) = result.move_handles.first().copied() else {
        return Err(GetSubObjectError::MissingHandle);
    };

    Ok(handle)
}

pub(crate) fn delete_user_save_data_all(
    service: &Session,
    uid: AccountUid,
) -> Result<u32, GetSubObjectError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const uid).cast::<u8>(), size_of::<AccountUid>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_DELETE_USER_SAVE_DATA_ALL)
        .in_raw(in_bytes)
        .send(&mut ipc_buf)
        .map_err(GetSubObjectError::Dispatch)?;

    let Some(handle) = result.move_handles.first().copied() else {
        return Err(GetSubObjectError::MissingHandle);
    };

    Ok(handle)
}

// ---------------------------------------------------------------------------
// Async commands (return move handle + copy handle)
// ---------------------------------------------------------------------------

pub(crate) fn request_application_update_info(
    service: &Session,
    application_id: u64,
) -> Result<AsyncOut, AsyncCommandError> {
    dispatch_async_u64_in(
        service,
        proto::APPMGR_REQUEST_APPLICATION_UPDATE_INFO,
        application_id,
    )
}

pub(crate) fn request_update_application2(
    service: &Session,
    application_id: u64,
) -> Result<AsyncOut, AsyncCommandError> {
    dispatch_async_u64_in(
        service,
        proto::APPMGR_REQUEST_UPDATE_APPLICATION2,
        application_id,
    )
}

pub(crate) fn request_download_application_control_data(
    service: &Session,
    application_id: u64,
) -> Result<AsyncOut, AsyncCommandError> {
    dispatch_async_u64_in(
        service,
        proto::APPMGR_REQUEST_DOWNLOAD_APPLICATION_CONTROL_DATA,
        application_id,
    )
}

pub(crate) fn request_check_game_card_registration(
    service: &Session,
    application_id: u64,
) -> Result<AsyncOut, AsyncCommandError> {
    dispatch_async_u64_in(
        service,
        proto::APPMGR_REQUEST_CHECK_GAME_CARD_REGISTRATION,
        application_id,
    )
}

pub(crate) fn request_download_application_prepurchased_rights(
    service: &Session,
    application_id: u64,
) -> Result<AsyncOut, AsyncCommandError> {
    dispatch_async_u64_in(
        service,
        proto::APPMGR_REQUEST_DOWNLOAD_APPLICATION_PREPURCHASED_RIGHTS,
        application_id,
    )
}

pub(crate) fn request_no_download_rights_error_resolution(
    service: &Session,
    application_id: u64,
) -> Result<AsyncOut, AsyncCommandError> {
    dispatch_async_u64_in(
        service,
        proto::APPMGR_REQUEST_NO_DOWNLOAD_RIGHTS_ERROR_RESOLUTION,
        application_id,
    )
}

pub(crate) fn request_resolve_no_download_rights_error(
    service: &Session,
    application_id: u64,
) -> Result<AsyncOut, AsyncCommandError> {
    dispatch_async_u64_in(
        service,
        proto::APPMGR_REQUEST_RESOLVE_NO_DOWNLOAD_RIGHTS_ERROR,
        application_id,
    )
}

pub(crate) fn request_game_card_registration_gold_point(
    service: &Session,
    input: GameCardRegistrationGoldPointIn,
) -> Result<AsyncOut, AsyncCommandError> {
    dispatch_async_in(
        service,
        proto::APPMGR_REQUEST_GAME_CARD_REGISTRATION_GOLD_POINT,
        input,
    )
}

pub(crate) fn request_register_game_card(
    service: &Session,
    input: RegisterGameCardIn,
) -> Result<AsyncOut, AsyncCommandError> {
    dispatch_async_in(service, proto::APPMGR_REQUEST_REGISTER_GAME_CARD, input)
}

pub(crate) fn request_ensure_download_task(
    service: &Session,
) -> Result<AsyncOut, AsyncCommandError> {
    dispatch_async_no_in(service, proto::APPMGR_REQUEST_ENSURE_DOWNLOAD_TASK)
}

pub(crate) fn request_download_task_list_data(
    service: &Session,
) -> Result<AsyncOut, AsyncCommandError> {
    dispatch_async_no_in(service, proto::APPMGR_REQUEST_DOWNLOAD_TASK_LIST_DATA)
}

pub(crate) fn request_verify_addon_contents_rights(
    service: &Session,
    application_id: u64,
) -> Result<AsyncOut, AsyncCommandError> {
    dispatch_async_u64_in(
        service,
        proto::APPMGR_REQUEST_VERIFY_ADDON_CONTENTS_RIGHTS,
        application_id,
    )
}

pub(crate) fn request_verify_application_deprecated(
    service: &Session,
    input: VerifyApplicationDeprecatedIn,
    tmem_handle: u32,
) -> Result<AsyncOut, AsyncCommandError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<VerifyApplicationDeprecatedIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_REQUEST_VERIFY_APPLICATION_DEPRECATED)
        .in_raw(in_bytes)
        .in_handle(tmem_handle)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AsyncCommandError::Dispatch)?;

    extract_async_out(&result)
}

pub(crate) fn request_verify_application(
    service: &Session,
    input: VerifyApplicationIn,
    tmem_handle: u32,
) -> Result<AsyncOut, AsyncCommandError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<VerifyApplicationIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_REQUEST_VERIFY_APPLICATION)
        .in_raw(in_bytes)
        .in_handle(tmem_handle)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AsyncCommandError::Dispatch)?;

    extract_async_out(&result)
}

/// ListApplicationTitle (cmd 407) — tmem-based async command.
pub(crate) fn list_application_title(
    service: &Session,
    input: ListApplicationTitleIn,
    tmem_handle: u32,
    app_ids: &[u8],
) -> Result<AsyncOut, AsyncCommandError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<ListApplicationTitleIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_LIST_APPLICATION_TITLE)
        .in_raw(in_bytes)
        .in_buffer(app_ids, BufferAttr::HIPC_MAP_ALIAS)
        .in_handle(tmem_handle)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AsyncCommandError::Dispatch)?;

    extract_async_out(&result)
}

/// ListApplicationIcon (cmd 408) — tmem-based async command.
pub(crate) fn list_application_icon(
    service: &Session,
    input: ListApplicationTitleIn,
    tmem_handle: u32,
    app_ids: &[u8],
) -> Result<AsyncOut, AsyncCommandError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<ListApplicationTitleIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::APPMGR_LIST_APPLICATION_ICON)
        .in_raw(in_bytes)
        .in_buffer(app_ids, BufferAttr::HIPC_MAP_ALIAS)
        .in_handle(tmem_handle)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AsyncCommandError::Dispatch)?;

    extract_async_out(&result)
}

// ---------------------------------------------------------------------------
// Shared dispatch helpers
// ---------------------------------------------------------------------------

/// Dispatches a command that returns a copy handle for an event.
fn acquire_event(service: &Session, cmd_id: u32) -> Result<u32, AcquireEventError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(cmd_id)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AcquireEventError::Dispatch)?;

    let Some(handle) = result.copy_handles.first().copied() else {
        return Err(AcquireEventError::MissingHandle);
    };

    Ok(handle)
}

/// Dispatches an async command with no input.
fn dispatch_async_no_in(service: &Session, cmd_id: u32) -> Result<AsyncOut, AsyncCommandError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(cmd_id)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AsyncCommandError::Dispatch)?;

    extract_async_out(&result)
}

/// Dispatches an async command with u64 input.
fn dispatch_async_u64_in(
    service: &Session,
    cmd_id: u32,
    input: u64,
) -> Result<AsyncOut, AsyncCommandError> {
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<u64>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AsyncCommandError::Dispatch)?;

    extract_async_out(&result)
}

/// Dispatches an async command with a Copy input struct.
fn dispatch_async_in<I: Copy>(
    service: &Session,
    cmd_id: u32,
    input: I,
) -> Result<AsyncOut, AsyncCommandError> {
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<I>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AsyncCommandError::Dispatch)?;

    extract_async_out(&result)
}

/// Extracts move + copy handles from a dispatch result.
pub(crate) fn extract_async_out(
    result: &nx_sf::service::DispatchResult<'_>,
) -> Result<AsyncOut, AsyncCommandError> {
    let Some(session_handle) = result.move_handles.first().copied() else {
        return Err(AsyncCommandError::MissingSessionHandle);
    };
    let Some(event_handle) = result.copy_handles.first().copied() else {
        return Err(AsyncCommandError::MissingEventHandle);
    };

    Ok(AsyncOut {
        session_handle,
        event_handle,
    })
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Result of an async command returning a sub-object session and event.
pub struct AsyncOut {
    pub session_handle: u32,
    pub event_handle: u32,
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned by event acquisition commands.
#[derive(Debug, thiserror::Error)]
pub enum AcquireEventError {
    #[error("failed to dispatch event acquisition")]
    Dispatch(#[source] DispatchError),
    #[error("event acquisition response did not include expected copy handle")]
    MissingHandle,
}

/// Error returned by async commands that return a sub-object and event.
#[derive(Debug, thiserror::Error)]
pub enum AsyncCommandError {
    #[error("async command dispatch failed")]
    Dispatch(#[source] DispatchError),
    #[error("async command response missing move handle")]
    MissingSessionHandle,
    #[error("async command response missing event copy handle")]
    MissingEventHandle,
}

/// Error returned by sub-object creation commands (move handle only).
#[derive(Debug, thiserror::Error)]
pub enum GetSubObjectError {
    #[error("failed to dispatch sub-object creation")]
    Dispatch(#[source] DispatchError),
    #[error("sub-object response did not include expected move handle")]
    MissingHandle,
}
