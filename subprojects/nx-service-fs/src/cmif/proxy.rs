use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainRef,
};

use crate::{
    dispatch::as_in_bytes,
    proto,
    savedata::{
        CreateSaveDataBySystemIdIn,
        CreateSaveDataIn,
        DeleteSaveDataByAttributeIn,
        DeleteSaveDataBySpaceIdIn,
        ExtendSaveDataIn,
        OpenSaveDataIn,
        OpenSaveDataInfoReaderWithFilterIn,
        ReadExtraDataBySpaceIdIn,
        WriteExtraDataIn,
    },
    types::*,
};

pub(crate) fn set_current_process(domain: DomainRef<'_>, ctx: u32) -> Result<(), DispatchError> {
    let pid_placeholder: u64 = 0;
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(proto::PROXY_SET_CURRENT_PROCESS)
        .context(ctx)
        .in_raw(as_in_bytes(&pid_placeholder))
        .send_pid()
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn open_file_system_legacy(
    domain: DomainRef<'_>,
    ctx: u32,
    fs_type: u32,
    content_path: &[u8; FS_MAX_PATH],
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_FILE_SYSTEM)
        .context(ctx)
        .in_raw(as_in_bytes(&fs_type))
        .in_buffer(content_path, BufferAttr::HIPC_POINTER)
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_data_file_system_by_current_process(
    domain: DomainRef<'_>,
    ctx: u32,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_DATA_FILE_SYSTEM_BY_CURRENT_PROCESS)
        .context(ctx)
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_file_system_with_patch(
    domain: DomainRef<'_>,
    ctx: u32,
    id: u64,
    fs_type: u32,
) -> Result<u32, DispatchError> {
    let input = OpenFileSystemWithPatchIn { fs_type, id };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_FILE_SYSTEM_WITH_PATCH)
        .context(ctx)
        .in_raw(as_in_bytes(&input))
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_file_system_with_id(
    domain: DomainRef<'_>,
    ctx: u32,
    id: u64,
    fs_type: u32,
    content_path: &[u8; FS_MAX_PATH],
) -> Result<u32, DispatchError> {
    let input = OpenFileSystemWithIdIn { fs_type, id };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_FILE_SYSTEM_WITH_ID)
        .context(ctx)
        .in_raw(as_in_bytes(&input))
        .in_buffer(content_path, BufferAttr::HIPC_POINTER)
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_file_system_with_id_v16(
    domain: DomainRef<'_>,
    ctx: u32,
    id: u64,
    fs_type: u32,
    attr: u8,
    content_path: &[u8; FS_MAX_PATH],
) -> Result<u32, DispatchError> {
    let input = OpenFileSystemWithIdV16In {
        attr,
        _pad: [0; 3],
        fs_type,
        id,
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_FILE_SYSTEM_WITH_ID_V16)
        .context(ctx)
        .in_raw(as_in_bytes(&input))
        .in_buffer(content_path, BufferAttr::HIPC_POINTER)
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_data_file_system_by_program_id(
    domain: DomainRef<'_>,
    ctx: u32,
    program_id: u64,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_DATA_FILE_SYSTEM_BY_PROGRAM_ID)
        .context(ctx)
        .in_raw(as_in_bytes(&program_id))
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_bis_file_system(
    domain: DomainRef<'_>,
    ctx: u32,
    partition_id: u32,
    path: &[u8; FS_MAX_PATH],
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_BIS_FILE_SYSTEM)
        .context(ctx)
        .in_raw(as_in_bytes(&partition_id))
        .in_buffer(path, BufferAttr::HIPC_POINTER)
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_bis_storage(
    domain: DomainRef<'_>,
    ctx: u32,
    partition_id: u32,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_BIS_STORAGE)
        .context(ctx)
        .in_raw(as_in_bytes(&partition_id))
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned storage object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_sd_card_file_system(
    domain: DomainRef<'_>,
    ctx: u32,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_SD_CARD_FILE_SYSTEM)
        .context(ctx)
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_host_file_system(
    domain: DomainRef<'_>,
    ctx: u32,
    path: &[u8; FS_MAX_PATH],
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_HOST_FILE_SYSTEM)
        .context(ctx)
        .in_buffer(path, BufferAttr::HIPC_POINTER)
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_host_file_system_with_option(
    domain: DomainRef<'_>,
    ctx: u32,
    path: &[u8; FS_MAX_PATH],
    flags: u32,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_HOST_FILE_SYSTEM_WITH_OPTION)
        .context(ctx)
        .in_raw(as_in_bytes(&flags))
        .in_buffer(path, BufferAttr::HIPC_POINTER)
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn delete_save_data_file_system(
    domain: DomainRef<'_>,
    ctx: u32,
    application_id: u64,
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(proto::PROXY_DELETE_SAVE_DATA_FILE_SYSTEM)
        .context(ctx)
        .in_raw(as_in_bytes(&application_id))
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn create_save_data_file_system_raw(
    domain: DomainRef<'_>,
    ctx: u32,
    input: &CreateSaveDataIn,
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(proto::PROXY_CREATE_SAVE_DATA_FILE_SYSTEM)
        .context(ctx)
        .in_raw(as_in_bytes(input))
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn create_save_data_file_system_by_system_save_data_id(
    domain: DomainRef<'_>,
    ctx: u32,
    input: &CreateSaveDataBySystemIdIn,
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(proto::PROXY_CREATE_SAVE_DATA_FILE_SYSTEM_BY_SYSTEM_SAVE_DATA_ID)
        .context(ctx)
        .in_raw(as_in_bytes(input))
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn delete_save_data_file_system_by_save_data_space_id(
    domain: DomainRef<'_>,
    ctx: u32,
    input: &DeleteSaveDataBySpaceIdIn,
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(proto::PROXY_DELETE_SAVE_DATA_FILE_SYSTEM_BY_SAVE_DATA_SPACE_ID)
        .context(ctx)
        .in_raw(as_in_bytes(input))
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn delete_save_data_file_system_by_save_data_attribute(
    domain: DomainRef<'_>,
    ctx: u32,
    input: &DeleteSaveDataByAttributeIn,
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(proto::PROXY_DELETE_SAVE_DATA_FILE_SYSTEM_BY_SAVE_DATA_ATTRIBUTE)
        .context(ctx)
        .in_raw(as_in_bytes(input))
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn is_exfat_supported(domain: DomainRef<'_>, ctx: u32) -> Result<bool, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = domain
        .dispatch(proto::PROXY_IS_EXFAT_SUPPORTED)
        .context(ctx)
        .out_size(size_of::<u8>())
        .send(&mut ipc_buf)?;
    Ok(result.data[0] & 1 != 0)
}

pub(crate) fn open_game_card_file_system(
    domain: DomainRef<'_>,
    ctx: u32,
    input: &OpenGameCardFileSystemIn,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_GAME_CARD_FILE_SYSTEM)
        .context(ctx)
        .in_raw(as_in_bytes(input))
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn extend_save_data_file_system(
    domain: DomainRef<'_>,
    ctx: u32,
    input: &ExtendSaveDataIn,
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(proto::PROXY_EXTEND_SAVE_DATA_FILE_SYSTEM)
        .context(ctx)
        .in_raw(as_in_bytes(input))
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn open_save_data_file_system(
    domain: DomainRef<'_>,
    ctx: u32,
    input: &OpenSaveDataIn,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_SAVE_DATA_FILE_SYSTEM)
        .context(ctx)
        .in_raw(as_in_bytes(input))
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_save_data_file_system_by_system_save_data_id(
    domain: DomainRef<'_>,
    ctx: u32,
    input: &OpenSaveDataIn,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_SAVE_DATA_FILE_SYSTEM_BY_SYSTEM_SAVE_DATA_ID)
        .context(ctx)
        .in_raw(as_in_bytes(input))
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_read_only_save_data_file_system(
    domain: DomainRef<'_>,
    ctx: u32,
    input: &OpenSaveDataIn,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_READ_ONLY_SAVE_DATA_FILE_SYSTEM)
        .context(ctx)
        .in_raw(as_in_bytes(input))
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn read_save_data_file_system_extra_data_by_save_data_space_id(
    domain: DomainRef<'_>,
    ctx: u32,
    input: &ReadExtraDataBySpaceIdIn,
    buf: &mut [u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(proto::PROXY_READ_SAVE_DATA_FILE_SYSTEM_EXTRA_DATA_BY_SAVE_DATA_SPACE_ID)
        .context(ctx)
        .in_raw(as_in_bytes(input))
        .out_buffer(buf, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn read_save_data_file_system_extra_data(
    domain: DomainRef<'_>,
    ctx: u32,
    save_id: u64,
    buf: &mut [u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(proto::PROXY_READ_SAVE_DATA_FILE_SYSTEM_EXTRA_DATA)
        .context(ctx)
        .in_raw(as_in_bytes(&save_id))
        .out_buffer(buf, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn write_save_data_file_system_extra_data(
    domain: DomainRef<'_>,
    ctx: u32,
    input: &WriteExtraDataIn,
    buf: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(proto::PROXY_WRITE_SAVE_DATA_FILE_SYSTEM_EXTRA_DATA)
        .context(ctx)
        .in_raw(as_in_bytes(input))
        .in_buffer(buf, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn open_save_data_info_reader(
    domain: DomainRef<'_>,
    ctx: u32,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_SAVE_DATA_INFO_READER)
        .context(ctx)
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned info reader object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_save_data_info_reader_by_save_data_space_id(
    domain: DomainRef<'_>,
    ctx: u32,
    space_id: u8,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_SAVE_DATA_INFO_READER_BY_SAVE_DATA_SPACE_ID)
        .context(ctx)
        .in_raw(as_in_bytes(&space_id))
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned info reader object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_save_data_info_reader_with_filter(
    domain: DomainRef<'_>,
    ctx: u32,
    input: &OpenSaveDataInfoReaderWithFilterIn,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_SAVE_DATA_INFO_READER_WITH_FILTER)
        .context(ctx)
        .in_raw(as_in_bytes(input))
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned info reader object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_image_directory_file_system(
    domain: DomainRef<'_>,
    ctx: u32,
    image_directory_id: u32,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_IMAGE_DIRECTORY_FILE_SYSTEM)
        .context(ctx)
        .in_raw(as_in_bytes(&image_directory_id))
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_content_storage_file_system(
    domain: DomainRef<'_>,
    ctx: u32,
    content_storage_id: u32,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_CONTENT_STORAGE_FILE_SYSTEM)
        .context(ctx)
        .in_raw(as_in_bytes(&content_storage_id))
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_custom_storage_file_system(
    domain: DomainRef<'_>,
    ctx: u32,
    custom_storage_id: u32,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_CUSTOM_STORAGE_FILE_SYSTEM)
        .context(ctx)
        .in_raw(as_in_bytes(&custom_storage_id))
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned filesystem object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_data_storage_by_current_process(
    domain: DomainRef<'_>,
    ctx: u32,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_DATA_STORAGE_BY_CURRENT_PROCESS)
        .context(ctx)
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned storage object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_data_storage_by_program_id(
    domain: DomainRef<'_>,
    ctx: u32,
    program_id: u64,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_DATA_STORAGE_BY_PROGRAM_ID)
        .context(ctx)
        .in_raw(as_in_bytes(&program_id))
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned storage object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_data_storage_by_data_id(
    domain: DomainRef<'_>,
    ctx: u32,
    input: &OpenDataStorageByDataIdIn,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_DATA_STORAGE_BY_DATA_ID)
        .context(ctx)
        .in_raw(as_in_bytes(input))
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned storage object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_patch_data_storage_by_current_process(
    domain: DomainRef<'_>,
    ctx: u32,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_PATCH_DATA_STORAGE_BY_CURRENT_PROCESS)
        .context(ctx)
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned storage object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_device_operator(domain: DomainRef<'_>, ctx: u32) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_DEVICE_OPERATOR)
        .context(ctx)
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned device operator object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn open_sd_card_detection_event_notifier(
    domain: DomainRef<'_>,
    ctx: u32,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::PROXY_OPEN_SD_CARD_DETECTION_EVENT_NOTIFIER)
        .context(ctx)
        .out_objects(1)
        .send(&mut ipc_buf)?;
    let object = result
        .take_object(0)
        .expect("server returned event notifier object");
    Ok(object.into_raw_object_id())
}

pub(crate) fn get_rights_id_by_path(
    domain: DomainRef<'_>,
    ctx: u32,
    path: &[u8; FS_MAX_PATH],
) -> Result<RightsId, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = domain
        .dispatch(proto::PROXY_GET_RIGHTS_ID_BY_PATH)
        .context(ctx)
        .out_size(size_of::<RightsId>())
        .in_buffer(path, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<RightsId>()) })
}

pub(crate) fn get_rights_id_and_key_generation_by_path(
    domain: DomainRef<'_>,
    ctx: u32,
    path: &[u8; FS_MAX_PATH],
    has_attr: bool,
    attr: u8,
) -> Result<GetRightsIdAndKeyGenOut, DispatchError> {
    if has_attr {
        let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

        let result = domain
            .dispatch(proto::PROXY_GET_RIGHTS_ID_AND_KEY_GENERATION_BY_PATH)
            .context(ctx)
            .in_raw(as_in_bytes(&attr))
            .out_size(size_of::<GetRightsIdAndKeyGenOut>())
            .in_buffer(path, BufferAttr::HIPC_POINTER)
            .send(&mut ipc_buf)?;
        Ok(unsafe {
            core::ptr::read_unaligned(result.data.as_ptr().cast::<GetRightsIdAndKeyGenOut>())
        })
    } else {
        let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

        let result = domain
            .dispatch(proto::PROXY_GET_RIGHTS_ID_AND_KEY_GENERATION_BY_PATH)
            .context(ctx)
            .out_size(size_of::<GetRightsIdAndKeyGenOut>())
            .in_buffer(path, BufferAttr::HIPC_POINTER)
            .send(&mut ipc_buf)?;
        Ok(unsafe {
            core::ptr::read_unaligned(result.data.as_ptr().cast::<GetRightsIdAndKeyGenOut>())
        })
    }
}

pub(crate) fn get_program_id(
    domain: DomainRef<'_>,
    ctx: u32,
    path: &[u8; FS_MAX_PATH],
    attr: u8,
) -> Result<u64, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = domain
        .dispatch(proto::PROXY_GET_PROGRAM_ID)
        .context(ctx)
        .in_raw(as_in_bytes(&attr))
        .out_size(size_of::<u64>())
        .in_buffer(path, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) })
}

pub(crate) fn is_signed_system_partition_on_sd_card_valid(
    domain: DomainRef<'_>,
    ctx: u32,
) -> Result<bool, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = domain
        .dispatch(proto::PROXY_IS_SIGNED_SYSTEM_PARTITION_ON_SD_CARD_VALID)
        .context(ctx)
        .out_size(size_of::<u8>())
        .send(&mut ipc_buf)?;
    Ok(result.data[0] & 1 != 0)
}

pub(crate) fn get_and_clear_error_info(
    domain: DomainRef<'_>,
    ctx: u32,
) -> Result<FileSystemProxyErrorInfo, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = domain
        .dispatch(proto::PROXY_GET_AND_CLEAR_ERROR_INFO)
        .context(ctx)
        .out_size(size_of::<FileSystemProxyErrorInfo>())
        .send(&mut ipc_buf)?;
    Ok(unsafe {
        core::ptr::read_unaligned(result.data.as_ptr().cast::<FileSystemProxyErrorInfo>())
    })
}

pub(crate) fn get_content_storage_info_index(
    domain: DomainRef<'_>,
    ctx: u32,
) -> Result<i32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = domain
        .dispatch(proto::PROXY_GET_CONTENT_STORAGE_INFO_INDEX)
        .context(ctx)
        .out_size(size_of::<u32>())
        .send(&mut ipc_buf)?;
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

pub(crate) fn disable_auto_save_data_creation(
    domain: DomainRef<'_>,
    ctx: u32,
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(proto::PROXY_DISABLE_AUTO_SAVE_DATA_CREATION)
        .context(ctx)
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn set_global_access_log_mode(
    domain: DomainRef<'_>,
    ctx: u32,
    mode: u32,
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(proto::PROXY_SET_GLOBAL_ACCESS_LOG_MODE)
        .context(ctx)
        .in_raw(as_in_bytes(&mode))
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn get_global_access_log_mode(
    domain: DomainRef<'_>,
    ctx: u32,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = domain
        .dispatch(proto::PROXY_GET_GLOBAL_ACCESS_LOG_MODE)
        .context(ctx)
        .out_size(size_of::<u32>())
        .send(&mut ipc_buf)?;
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}

pub(crate) fn output_access_log_to_sd_card(
    domain: DomainRef<'_>,
    ctx: u32,
    log: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(proto::PROXY_OUTPUT_ACCESS_LOG_TO_SD_CARD)
        .context(ctx)
        .in_buffer(log, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

pub(crate) fn get_program_index_for_access_log(
    domain: DomainRef<'_>,
    ctx: u32,
) -> Result<ProgramIndexForAccessLogOut, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = domain
        .dispatch(proto::PROXY_GET_PROGRAM_INDEX_FOR_ACCESS_LOG)
        .context(ctx)
        .out_size(size_of::<ProgramIndexForAccessLogOut>())
        .send(&mut ipc_buf)?;
    Ok(unsafe {
        core::ptr::read_unaligned(result.data.as_ptr().cast::<ProgramIndexForAccessLogOut>())
    })
}

pub(crate) fn get_and_clear_memory_report_info(
    domain: DomainRef<'_>,
    ctx: u32,
) -> Result<MemoryReportInfo, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = domain
        .dispatch(proto::PROXY_GET_AND_CLEAR_MEMORY_REPORT_INFO)
        .context(ctx)
        .out_size(size_of::<MemoryReportInfo>())
        .send(&mut ipc_buf)?;
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<MemoryReportInfo>()) })
}
