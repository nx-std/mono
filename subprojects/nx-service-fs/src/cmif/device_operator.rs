use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainObjectRef,
};
use zerocopy::IntoBytes as _;

use crate::{
    diagnostics::GetAndClearStorageErrorInfoOut,
    dispatch::{
        dispatch_in_out,
        dispatch_in_size_out_buffer,
        dispatch_out,
        dispatch_out_bool,
        dispatch_out_i64,
        dispatch_out_u32,
    },
    gamecard::{
        GameCardErrorReportInfo,
        GameCardHandle,
        GameCardUpdatePartitionInfo,
        GetDeviceCertIn,
    },
    proto,
};

pub(crate) fn is_sd_card_inserted(
    object: DomainObjectRef<'_>,
    ctx: u32,
) -> Result<bool, DispatchError> {
    dispatch_out_bool(object, proto::DEVICE_OPERATOR_IS_SD_CARD_INSERTED, ctx)
}

pub(crate) fn get_sd_card_speed_mode(
    object: DomainObjectRef<'_>,
    ctx: u32,
) -> Result<i64, DispatchError> {
    dispatch_out_i64(object, proto::DEVICE_OPERATOR_GET_SD_CARD_SPEED_MODE, ctx)
}

pub(crate) fn get_sd_card_cid(
    object: DomainObjectRef<'_>,
    ctx: u32,
    dst: &mut [u8],
    size: i64,
) -> Result<(), DispatchError> {
    dispatch_in_size_out_buffer(
        object,
        proto::DEVICE_OPERATOR_GET_SD_CARD_CID,
        ctx,
        size,
        dst,
    )
}

pub(crate) fn get_sd_card_user_area_size(
    object: DomainObjectRef<'_>,
    ctx: u32,
) -> Result<i64, DispatchError> {
    dispatch_out_i64(
        object,
        proto::DEVICE_OPERATOR_GET_SD_CARD_USER_AREA_SIZE,
        ctx,
    )
}

pub(crate) fn get_sd_card_protected_area_size(
    object: DomainObjectRef<'_>,
    ctx: u32,
) -> Result<i64, DispatchError> {
    dispatch_out_i64(
        object,
        proto::DEVICE_OPERATOR_GET_SD_CARD_PROTECTED_AREA_SIZE,
        ctx,
    )
}

pub(crate) fn get_and_clear_storage_error_info(
    object: DomainObjectRef<'_>,
    ctx: u32,
    cmd_id: u32,
    size: i64,
    dst: &mut [u8],
) -> Result<GetAndClearStorageErrorInfoOut, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(cmd_id)
        .context(ctx)
        .in_raw(size.as_bytes())
        .out_size(size_of::<GetAndClearStorageErrorInfoOut>())
        .out_buffer(dst, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<GetAndClearStorageErrorInfoOut>())
}

pub(crate) fn get_mmc_cid(
    object: DomainObjectRef<'_>,
    ctx: u32,
    dst: &mut [u8],
    size: i64,
) -> Result<(), DispatchError> {
    dispatch_in_size_out_buffer(object, proto::DEVICE_OPERATOR_GET_MMC_CID, ctx, size, dst)
}

pub(crate) fn get_mmc_speed_mode(
    object: DomainObjectRef<'_>,
    ctx: u32,
) -> Result<i64, DispatchError> {
    dispatch_out_i64(object, proto::DEVICE_OPERATOR_GET_MMC_SPEED_MODE, ctx)
}

pub(crate) fn get_mmc_patrol_count(
    object: DomainObjectRef<'_>,
    ctx: u32,
) -> Result<u32, DispatchError> {
    dispatch_out_u32(object, proto::DEVICE_OPERATOR_GET_MMC_PATROL_COUNT, ctx)
}

pub(crate) fn get_mmc_extended_csd(
    object: DomainObjectRef<'_>,
    ctx: u32,
    dst: &mut [u8],
    size: i64,
) -> Result<(), DispatchError> {
    dispatch_in_size_out_buffer(
        object,
        proto::DEVICE_OPERATOR_GET_MMC_EXTENDED_CSD,
        ctx,
        size,
        dst,
    )
}

pub(crate) fn is_game_card_inserted(
    object: DomainObjectRef<'_>,
    ctx: u32,
) -> Result<bool, DispatchError> {
    dispatch_out_bool(object, proto::DEVICE_OPERATOR_IS_GAME_CARD_INSERTED, ctx)
}

pub(crate) fn get_game_card_handle(
    object: DomainObjectRef<'_>,
    ctx: u32,
) -> Result<GameCardHandle, DispatchError> {
    dispatch_out(object, proto::DEVICE_OPERATOR_GET_GAME_CARD_HANDLE, ctx)
}

pub(crate) fn get_game_card_update_partition_info(
    object: DomainObjectRef<'_>,
    ctx: u32,
    handle: &GameCardHandle,
) -> Result<GameCardUpdatePartitionInfo, DispatchError> {
    dispatch_in_out(
        object,
        proto::DEVICE_OPERATOR_GET_GAME_CARD_UPDATE_PARTITION_INFO,
        ctx,
        *handle,
    )
}

pub(crate) fn get_game_card_attribute(
    object: DomainObjectRef<'_>,
    ctx: u32,
    handle: &GameCardHandle,
) -> Result<u8, DispatchError> {
    dispatch_in_out(
        object,
        proto::DEVICE_OPERATOR_GET_GAME_CARD_ATTRIBUTE,
        ctx,
        *handle,
    )
}

pub(crate) fn get_game_card_device_certificate_legacy(
    object: DomainObjectRef<'_>,
    ctx: u32,
    handle: &GameCardHandle,
    size: i64,
    dst: &mut [u8],
) -> Result<i64, DispatchError> {
    let input = GetDeviceCertIn {
        handle: *handle,
        _pad: 0,
        buffer_size: size,
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(proto::DEVICE_OPERATOR_GET_GAME_CARD_DEVICE_CERTIFICATE)
        .context(ctx)
        .in_raw(input.as_bytes())
        .out_buffer(dst, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| 0x200)
}

pub(crate) fn get_game_card_device_certificate(
    object: DomainObjectRef<'_>,
    ctx: u32,
    handle: &GameCardHandle,
    size: i64,
    dst: &mut [u8],
) -> Result<i64, DispatchError> {
    let input = GetDeviceCertIn {
        handle: *handle,
        _pad: 0,
        buffer_size: size,
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::DEVICE_OPERATOR_GET_GAME_CARD_DEVICE_CERTIFICATE)
        .context(ctx)
        .in_raw(input.as_bytes())
        .out_size(size_of::<i64>())
        .out_buffer(dst, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<i64>())
}

pub(crate) fn get_game_card_id_set(
    object: DomainObjectRef<'_>,
    ctx: u32,
    dst: &mut [u8],
    size: i64,
) -> Result<(), DispatchError> {
    dispatch_in_size_out_buffer(
        object,
        proto::DEVICE_OPERATOR_GET_GAME_CARD_ID_SET,
        ctx,
        size,
        dst,
    )
}

pub(crate) fn get_game_card_error_report_info(
    object: DomainObjectRef<'_>,
    ctx: u32,
) -> Result<GameCardErrorReportInfo, DispatchError> {
    dispatch_out(
        object,
        proto::DEVICE_OPERATOR_GET_GAME_CARD_ERROR_REPORT_INFO,
        ctx,
    )
}

pub(crate) fn get_game_card_device_id(
    object: DomainObjectRef<'_>,
    ctx: u32,
    dst: &mut [u8],
    size: i64,
) -> Result<(), DispatchError> {
    dispatch_in_size_out_buffer(
        object,
        proto::DEVICE_OPERATOR_GET_GAME_CARD_DEVICE_ID,
        ctx,
        size,
        dst,
    )
}

pub(crate) fn challenge_card_existence(
    object: DomainObjectRef<'_>,
    ctx: u32,
    handle: &GameCardHandle,
    dst: &mut [u8],
    seed: &[u8],
    value: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(proto::DEVICE_OPERATOR_CHALLENGE_CARD_EXISTENCE)
        .context(ctx)
        .in_raw(handle.as_bytes())
        .out_buffer(dst, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(seed, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(value, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}
