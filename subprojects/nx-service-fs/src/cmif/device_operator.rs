use core::mem::{ManuallyDrop, size_of};

use nx_sf::service::{BufferAttr, DispatchError, DomainObject};

use crate::{
    dispatch::{
        dispatch_in_out, dispatch_in_size_out_buffer, dispatch_out, dispatch_out_bool,
        dispatch_out_i64, dispatch_out_u32,
    },
    proto,
    types::*,
};

fn as_in_bytes<I: Copy>(input: &I) -> &[u8] {
    unsafe { core::slice::from_raw_parts((&raw const *input).cast::<u8>(), size_of::<I>()) }
}

pub(crate) fn is_sd_card_inserted(
    object: &ManuallyDrop<DomainObject<'_>>,
    ctx: u32,
) -> Result<bool, DispatchError> {
    dispatch_out_bool(object, proto::DEVICE_OPERATOR_IS_SD_CARD_INSERTED, ctx)
}

pub(crate) fn get_sd_card_speed_mode(
    object: &ManuallyDrop<DomainObject<'_>>,
    ctx: u32,
) -> Result<i64, DispatchError> {
    dispatch_out_i64(object, proto::DEVICE_OPERATOR_GET_SD_CARD_SPEED_MODE, ctx)
}

pub(crate) fn get_sd_card_cid(
    object: &ManuallyDrop<DomainObject<'_>>,
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
    object: &ManuallyDrop<DomainObject<'_>>,
    ctx: u32,
) -> Result<i64, DispatchError> {
    dispatch_out_i64(
        object,
        proto::DEVICE_OPERATOR_GET_SD_CARD_USER_AREA_SIZE,
        ctx,
    )
}

pub(crate) fn get_sd_card_protected_area_size(
    object: &ManuallyDrop<DomainObject<'_>>,
    ctx: u32,
) -> Result<i64, DispatchError> {
    dispatch_out_i64(
        object,
        proto::DEVICE_OPERATOR_GET_SD_CARD_PROTECTED_AREA_SIZE,
        ctx,
    )
}

pub(crate) fn get_and_clear_storage_error_info(
    object: &ManuallyDrop<DomainObject<'_>>,
    ctx: u32,
    cmd_id: u32,
    size: i64,
    dst: &mut [u8],
) -> Result<GetAndClearStorageErrorInfoOut, DispatchError> {
    let result = object
        .dispatch(cmd_id)
        .context(ctx)
        .in_raw(as_in_bytes(&size))
        .out_size(size_of::<GetAndClearStorageErrorInfoOut>())
        .out_buffer(dst, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;
    Ok(unsafe {
        core::ptr::read_unaligned(
            result
                .data
                .as_ptr()
                .cast::<GetAndClearStorageErrorInfoOut>(),
        )
    })
}

pub(crate) fn get_mmc_cid(
    object: &ManuallyDrop<DomainObject<'_>>,
    ctx: u32,
    dst: &mut [u8],
    size: i64,
) -> Result<(), DispatchError> {
    dispatch_in_size_out_buffer(object, proto::DEVICE_OPERATOR_GET_MMC_CID, ctx, size, dst)
}

pub(crate) fn get_mmc_speed_mode(
    object: &ManuallyDrop<DomainObject<'_>>,
    ctx: u32,
) -> Result<i64, DispatchError> {
    dispatch_out_i64(object, proto::DEVICE_OPERATOR_GET_MMC_SPEED_MODE, ctx)
}

pub(crate) fn get_mmc_patrol_count(
    object: &ManuallyDrop<DomainObject<'_>>,
    ctx: u32,
) -> Result<u32, DispatchError> {
    dispatch_out_u32(object, proto::DEVICE_OPERATOR_GET_MMC_PATROL_COUNT, ctx)
}

pub(crate) fn get_mmc_extended_csd(
    object: &ManuallyDrop<DomainObject<'_>>,
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
    object: &ManuallyDrop<DomainObject<'_>>,
    ctx: u32,
) -> Result<bool, DispatchError> {
    dispatch_out_bool(object, proto::DEVICE_OPERATOR_IS_GAME_CARD_INSERTED, ctx)
}

pub(crate) fn get_game_card_handle(
    object: &ManuallyDrop<DomainObject<'_>>,
    ctx: u32,
) -> Result<GameCardHandle, DispatchError> {
    dispatch_out(object, proto::DEVICE_OPERATOR_GET_GAME_CARD_HANDLE, ctx)
}

pub(crate) fn get_game_card_update_partition_info(
    object: &ManuallyDrop<DomainObject<'_>>,
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
    object: &ManuallyDrop<DomainObject<'_>>,
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
    object: &ManuallyDrop<DomainObject<'_>>,
    ctx: u32,
    handle: &GameCardHandle,
    size: i64,
    dst: &mut [u8],
) -> Result<i64, DispatchError> {
    let input = GetDeviceCertIn {
        handle: *handle,
        buffer_size: size,
    };
    object
        .dispatch(proto::DEVICE_OPERATOR_GET_GAME_CARD_DEVICE_CERTIFICATE)
        .context(ctx)
        .in_raw(as_in_bytes(&input))
        .out_buffer(dst, BufferAttr::HIPC_MAP_ALIAS)
        .send()
        .map(|_| 0x200)
}

pub(crate) fn get_game_card_device_certificate(
    object: &ManuallyDrop<DomainObject<'_>>,
    ctx: u32,
    handle: &GameCardHandle,
    size: i64,
    dst: &mut [u8],
) -> Result<i64, DispatchError> {
    let input = GetDeviceCertIn {
        handle: *handle,
        buffer_size: size,
    };
    let result = object
        .dispatch(proto::DEVICE_OPERATOR_GET_GAME_CARD_DEVICE_CERTIFICATE)
        .context(ctx)
        .in_raw(as_in_bytes(&input))
        .out_size(size_of::<i64>())
        .out_buffer(dst, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<i64>()) })
}

pub(crate) fn get_game_card_id_set(
    object: &ManuallyDrop<DomainObject<'_>>,
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
    object: &ManuallyDrop<DomainObject<'_>>,
    ctx: u32,
) -> Result<GameCardErrorReportInfo, DispatchError> {
    dispatch_out(
        object,
        proto::DEVICE_OPERATOR_GET_GAME_CARD_ERROR_REPORT_INFO,
        ctx,
    )
}

pub(crate) fn get_game_card_device_id(
    object: &ManuallyDrop<DomainObject<'_>>,
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
    object: &ManuallyDrop<DomainObject<'_>>,
    ctx: u32,
    handle: &GameCardHandle,
    dst: &mut [u8],
    seed: &[u8],
    value: &[u8],
) -> Result<(), DispatchError> {
    object
        .dispatch(proto::DEVICE_OPERATOR_CHALLENGE_CARD_EXISTENCE)
        .context(ctx)
        .in_raw(as_in_bytes(handle))
        .out_buffer(dst, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(seed, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(value, BufferAttr::HIPC_MAP_ALIAS)
        .send()
        .map(|_| ())
}
