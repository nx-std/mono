//! Custom button config commands (cmds 1250-1291, 10.0.0+).
//!
//! These commands manage per-controller button remapping presets stored in
//! system settings. Paired deprecated/current variants per IC-4.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Session};

use crate::{
    dispatch::{dispatch_in, dispatch_in_out, dispatch_no_io, dispatch_out},
    proto,
    types::{
        HidcfgButtonConfigEmbedded, HidcfgButtonConfigFull, HidcfgButtonConfigLeft,
        HidcfgButtonConfigRight, HidcfgStorageName, InU64BoolIn, UniquePadId,
    },
};

// ---------------------------------------------------------------------------
// Query commands
// ---------------------------------------------------------------------------

/// IsCustomButtonConfigSupported (cmd 1250, 10.0.0+).
pub(crate) fn is_custom_button_config_supported(
    service: &Session,
    unique_pad_id: UniquePadId,
) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_in_out(
        service,
        proto::IS_CUSTOM_BUTTON_CONFIG_SUPPORTED,
        &unique_pad_id.id,
    )?;
    Ok(out & 1 != 0)
}

/// IsDefaultButtonConfigEmbedded (cmd 1251, 10.0.0+).
pub(crate) fn is_default_button_config_embedded(
    service: &Session,
    config: &HidcfgButtonConfigEmbedded,
) -> Result<bool, DispatchError> {
    dispatch_in_buf_out_bool(service, config, proto::IS_DEFAULT_BUTTON_CONFIG_EMBEDDED)
}

/// IsDefaultButtonConfigFull (cmd 1252, 10.0.0+).
pub(crate) fn is_default_button_config_full(
    service: &Session,
    config: &HidcfgButtonConfigFull,
) -> Result<bool, DispatchError> {
    dispatch_in_buf_out_bool(service, config, proto::IS_DEFAULT_BUTTON_CONFIG_FULL)
}

/// IsDefaultButtonConfigLeft (cmd 1253, 10.0.0+).
pub(crate) fn is_default_button_config_left(
    service: &Session,
    config: &HidcfgButtonConfigLeft,
) -> Result<bool, DispatchError> {
    dispatch_in_buf_out_bool(service, config, proto::IS_DEFAULT_BUTTON_CONFIG_LEFT)
}

/// IsDefaultButtonConfigRight (cmd 1254, 10.0.0+).
pub(crate) fn is_default_button_config_right(
    service: &Session,
    config: &HidcfgButtonConfigRight,
) -> Result<bool, DispatchError> {
    dispatch_in_buf_out_bool(service, config, proto::IS_DEFAULT_BUTTON_CONFIG_RIGHT)
}

/// IsButtonConfigStorageEmbeddedEmpty (cmd 1255, 10.0.0+).
pub(crate) fn is_button_config_storage_embedded_empty(
    service: &Session,
    index: i32,
) -> Result<bool, DispatchError> {
    dispatch_in_u32_out_bool(
        service,
        index as u32,
        proto::IS_BUTTON_CONFIG_STORAGE_EMBEDDED_EMPTY,
    )
}

/// IsButtonConfigStorageFullEmpty (cmd 1256, 10.0.0+).
pub(crate) fn is_button_config_storage_full_empty(
    service: &Session,
    index: i32,
) -> Result<bool, DispatchError> {
    dispatch_in_u32_out_bool(
        service,
        index as u32,
        proto::IS_BUTTON_CONFIG_STORAGE_FULL_EMPTY,
    )
}

/// IsButtonConfigStorageLeftEmpty (cmd 1257, 10.0.0+).
pub(crate) fn is_button_config_storage_left_empty(
    service: &Session,
    index: i32,
) -> Result<bool, DispatchError> {
    dispatch_in_u32_out_bool(
        service,
        index as u32,
        proto::IS_BUTTON_CONFIG_STORAGE_LEFT_EMPTY,
    )
}

/// IsButtonConfigStorageRightEmpty (cmd 1258, 10.0.0+).
pub(crate) fn is_button_config_storage_right_empty(
    service: &Session,
    index: i32,
) -> Result<bool, DispatchError> {
    dispatch_in_u32_out_bool(
        service,
        index as u32,
        proto::IS_BUTTON_CONFIG_STORAGE_RIGHT_EMPTY,
    )
}

// ---------------------------------------------------------------------------
// Deprecated storage get/set [10.0.0-12.1.0]
// ---------------------------------------------------------------------------

/// GetButtonConfigStorageEmbeddedDeprecated (cmd 1259, 10.0.0-12.1.0).
pub(crate) fn get_button_config_storage_embedded_deprecated(
    service: &Session,
    index: i32,
    config: &mut HidcfgButtonConfigEmbedded,
) -> Result<(), DispatchError> {
    dispatch_in_u32_out_buf_fixed(
        service,
        index as u32,
        config,
        proto::GET_BUTTON_CONFIG_STORAGE_EMBEDDED_DEPRECATED,
    )
}

/// GetButtonConfigStorageFullDeprecated (cmd 1260, 10.0.0-12.1.0).
pub(crate) fn get_button_config_storage_full_deprecated(
    service: &Session,
    index: i32,
    config: &mut HidcfgButtonConfigFull,
) -> Result<(), DispatchError> {
    dispatch_in_u32_out_buf_fixed(
        service,
        index as u32,
        config,
        proto::GET_BUTTON_CONFIG_STORAGE_FULL_DEPRECATED,
    )
}

/// GetButtonConfigStorageLeftDeprecated (cmd 1261, 10.0.0-12.1.0).
pub(crate) fn get_button_config_storage_left_deprecated(
    service: &Session,
    index: i32,
    config: &mut HidcfgButtonConfigLeft,
) -> Result<(), DispatchError> {
    dispatch_in_u32_out_buf_fixed(
        service,
        index as u32,
        config,
        proto::GET_BUTTON_CONFIG_STORAGE_LEFT_DEPRECATED,
    )
}

/// GetButtonConfigStorageRightDeprecated (cmd 1262, 10.0.0-12.1.0).
pub(crate) fn get_button_config_storage_right_deprecated(
    service: &Session,
    index: i32,
    config: &mut HidcfgButtonConfigRight,
) -> Result<(), DispatchError> {
    dispatch_in_u32_out_buf_fixed(
        service,
        index as u32,
        config,
        proto::GET_BUTTON_CONFIG_STORAGE_RIGHT_DEPRECATED,
    )
}

/// SetButtonConfigStorageEmbeddedDeprecated (cmd 1263, 10.0.0-12.1.0).
pub(crate) fn set_button_config_storage_embedded_deprecated(
    service: &Session,
    index: i32,
    config: &HidcfgButtonConfigEmbedded,
) -> Result<(), DispatchError> {
    dispatch_in_u32_in_buf_fixed(
        service,
        index as u32,
        config,
        proto::SET_BUTTON_CONFIG_STORAGE_EMBEDDED_DEPRECATED,
    )
}

/// SetButtonConfigStorageFullDeprecated (cmd 1264, 10.0.0-12.1.0).
pub(crate) fn set_button_config_storage_full_deprecated(
    service: &Session,
    index: i32,
    config: &HidcfgButtonConfigFull,
) -> Result<(), DispatchError> {
    dispatch_in_u32_in_buf_fixed(
        service,
        index as u32,
        config,
        proto::SET_BUTTON_CONFIG_STORAGE_FULL_DEPRECATED,
    )
}

/// SetButtonConfigStorageLeftDeprecated (cmd 1265, 10.0.0-12.1.0).
pub(crate) fn set_button_config_storage_left_deprecated(
    service: &Session,
    index: i32,
    config: &HidcfgButtonConfigLeft,
) -> Result<(), DispatchError> {
    dispatch_in_u32_in_buf_fixed(
        service,
        index as u32,
        config,
        proto::SET_BUTTON_CONFIG_STORAGE_LEFT_DEPRECATED,
    )
}

/// SetButtonConfigStorageRightDeprecated (cmd 1266, 10.0.0-12.1.0).
pub(crate) fn set_button_config_storage_right_deprecated(
    service: &Session,
    index: i32,
    config: &HidcfgButtonConfigRight,
) -> Result<(), DispatchError> {
    dispatch_in_u32_in_buf_fixed(
        service,
        index as u32,
        config,
        proto::SET_BUTTON_CONFIG_STORAGE_RIGHT_DEPRECATED,
    )
}

// ---------------------------------------------------------------------------
// Delete storage [10.0.0+]
// ---------------------------------------------------------------------------

/// DeleteButtonConfigStorageEmbedded (cmd 1267, 10.0.0+).
pub(crate) fn delete_button_config_storage_embedded(
    service: &Session,
    index: i32,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::DELETE_BUTTON_CONFIG_STORAGE_EMBEDDED,
        &(index as u32),
    )
}

/// DeleteButtonConfigStorageFull (cmd 1268, 10.0.0+).
pub(crate) fn delete_button_config_storage_full(
    service: &Session,
    index: i32,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::DELETE_BUTTON_CONFIG_STORAGE_FULL,
        &(index as u32),
    )
}

/// DeleteButtonConfigStorageLeft (cmd 1269, 10.0.0+).
pub(crate) fn delete_button_config_storage_left(
    service: &Session,
    index: i32,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::DELETE_BUTTON_CONFIG_STORAGE_LEFT,
        &(index as u32),
    )
}

/// DeleteButtonConfigStorageRight (cmd 1270, 10.0.0+).
pub(crate) fn delete_button_config_storage_right(
    service: &Session,
    index: i32,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::DELETE_BUTTON_CONFIG_STORAGE_RIGHT,
        &(index as u32),
    )
}

// ---------------------------------------------------------------------------
// Custom config control [10.0.0+]
// ---------------------------------------------------------------------------

/// IsUsingCustomButtonConfig (cmd 1271, 10.0.0+).
pub(crate) fn is_using_custom_button_config(
    service: &Session,
    unique_pad_id: UniquePadId,
) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_in_out(
        service,
        proto::IS_USING_CUSTOM_BUTTON_CONFIG,
        &unique_pad_id.id,
    )?;
    Ok(out & 1 != 0)
}

/// IsAnyCustomButtonConfigEnabled (cmd 1272, 10.0.0+).
pub(crate) fn is_any_custom_button_config_enabled(
    service: &Session,
) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_out(service, proto::IS_ANY_CUSTOM_BUTTON_CONFIG_ENABLED)?;
    Ok(out & 1 != 0)
}

/// SetAllCustomButtonConfigEnabled (cmd 1273, 10.0.0+).
pub(crate) fn set_all_custom_button_config_enabled(
    service: &Session,
    aruid: u64,
    flag: bool,
) -> Result<(), DispatchError> {
    let input = InU64BoolIn {
        flag: flag as u8,
        pad: [0; 7],
        value: aruid,
    };
    dispatch_in(service, proto::SET_ALL_CUSTOM_BUTTON_CONFIG_ENABLED, &input)
}

/// SetDefaultButtonConfig (cmd 1274, 10.0.0+).
pub(crate) fn set_default_button_config(
    service: &Session,
    unique_pad_id: UniquePadId,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_DEFAULT_BUTTON_CONFIG, &unique_pad_id.id)
}

/// SetAllDefaultButtonConfig (cmd 1275, 10.0.0+).
pub(crate) fn set_all_default_button_config(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::SET_ALL_DEFAULT_BUTTON_CONFIG)
}

// ---------------------------------------------------------------------------
// Hid button config (runtime per-pad) [10.0.0+]
// ---------------------------------------------------------------------------

/// SetHidButtonConfigEmbedded (cmd 1276, 10.0.0+).
pub(crate) fn set_hid_button_config_embedded(
    service: &Session,
    unique_pad_id: UniquePadId,
    config: &HidcfgButtonConfigEmbedded,
) -> Result<(), DispatchError> {
    dispatch_in_u64_in_buf_fixed(
        service,
        unique_pad_id.id,
        config,
        proto::SET_HID_BUTTON_CONFIG_EMBEDDED,
    )
}

/// SetHidButtonConfigFull (cmd 1277, 10.0.0+).
pub(crate) fn set_hid_button_config_full(
    service: &Session,
    unique_pad_id: UniquePadId,
    config: &HidcfgButtonConfigFull,
) -> Result<(), DispatchError> {
    dispatch_in_u64_in_buf_fixed(
        service,
        unique_pad_id.id,
        config,
        proto::SET_HID_BUTTON_CONFIG_FULL,
    )
}

/// SetHidButtonConfigLeft (cmd 1278, 10.0.0+).
pub(crate) fn set_hid_button_config_left(
    service: &Session,
    unique_pad_id: UniquePadId,
    config: &HidcfgButtonConfigLeft,
) -> Result<(), DispatchError> {
    dispatch_in_u64_in_buf_fixed(
        service,
        unique_pad_id.id,
        config,
        proto::SET_HID_BUTTON_CONFIG_LEFT,
    )
}

/// SetHidButtonConfigRight (cmd 1279, 10.0.0+).
pub(crate) fn set_hid_button_config_right(
    service: &Session,
    unique_pad_id: UniquePadId,
    config: &HidcfgButtonConfigRight,
) -> Result<(), DispatchError> {
    dispatch_in_u64_in_buf_fixed(
        service,
        unique_pad_id.id,
        config,
        proto::SET_HID_BUTTON_CONFIG_RIGHT,
    )
}

/// GetHidButtonConfigEmbedded (cmd 1280, 10.0.0+).
pub(crate) fn get_hid_button_config_embedded(
    service: &Session,
    unique_pad_id: UniquePadId,
    config: &mut HidcfgButtonConfigEmbedded,
) -> Result<(), DispatchError> {
    dispatch_in_u64_out_buf_fixed(
        service,
        unique_pad_id.id,
        config,
        proto::GET_HID_BUTTON_CONFIG_EMBEDDED,
    )
}

/// GetHidButtonConfigFull (cmd 1281, 10.0.0+).
pub(crate) fn get_hid_button_config_full(
    service: &Session,
    unique_pad_id: UniquePadId,
    config: &mut HidcfgButtonConfigFull,
) -> Result<(), DispatchError> {
    dispatch_in_u64_out_buf_fixed(
        service,
        unique_pad_id.id,
        config,
        proto::GET_HID_BUTTON_CONFIG_FULL,
    )
}

/// GetHidButtonConfigLeft (cmd 1282, 10.0.0+).
pub(crate) fn get_hid_button_config_left(
    service: &Session,
    unique_pad_id: UniquePadId,
    config: &mut HidcfgButtonConfigLeft,
) -> Result<(), DispatchError> {
    dispatch_in_u64_out_buf_fixed(
        service,
        unique_pad_id.id,
        config,
        proto::GET_HID_BUTTON_CONFIG_LEFT,
    )
}

/// GetHidButtonConfigRight (cmd 1283, 10.0.0+).
pub(crate) fn get_hid_button_config_right(
    service: &Session,
    unique_pad_id: UniquePadId,
    config: &mut HidcfgButtonConfigRight,
) -> Result<(), DispatchError> {
    dispatch_in_u64_out_buf_fixed(
        service,
        unique_pad_id.id,
        config,
        proto::GET_HID_BUTTON_CONFIG_RIGHT,
    )
}

// ---------------------------------------------------------------------------
// Named storage get/set [11.0.0+]
// ---------------------------------------------------------------------------

/// GetButtonConfigStorageEmbedded (cmd 1284, 11.0.0+).
pub(crate) fn get_button_config_storage_embedded(
    service: &Session,
    index: i32,
    config: &mut HidcfgButtonConfigEmbedded,
    name: &mut HidcfgStorageName,
) -> Result<(), DispatchError> {
    dispatch_in_u32_out_buf_fixed_out_ptr_fixed(
        service,
        index as u32,
        config,
        name,
        proto::GET_BUTTON_CONFIG_STORAGE_EMBEDDED,
    )
}

/// GetButtonConfigStorageFull (cmd 1285, 11.0.0+).
pub(crate) fn get_button_config_storage_full(
    service: &Session,
    index: i32,
    config: &mut HidcfgButtonConfigFull,
    name: &mut HidcfgStorageName,
) -> Result<(), DispatchError> {
    dispatch_in_u32_out_buf_fixed_out_ptr_fixed(
        service,
        index as u32,
        config,
        name,
        proto::GET_BUTTON_CONFIG_STORAGE_FULL,
    )
}

/// GetButtonConfigStorageLeft (cmd 1286, 11.0.0+).
pub(crate) fn get_button_config_storage_left(
    service: &Session,
    index: i32,
    config: &mut HidcfgButtonConfigLeft,
    name: &mut HidcfgStorageName,
) -> Result<(), DispatchError> {
    dispatch_in_u32_out_buf_fixed_out_ptr_fixed(
        service,
        index as u32,
        config,
        name,
        proto::GET_BUTTON_CONFIG_STORAGE_LEFT,
    )
}

/// GetButtonConfigStorageRight (cmd 1287, 11.0.0+).
pub(crate) fn get_button_config_storage_right(
    service: &Session,
    index: i32,
    config: &mut HidcfgButtonConfigRight,
    name: &mut HidcfgStorageName,
) -> Result<(), DispatchError> {
    dispatch_in_u32_out_buf_fixed_out_ptr_fixed(
        service,
        index as u32,
        config,
        name,
        proto::GET_BUTTON_CONFIG_STORAGE_RIGHT,
    )
}

/// SetButtonConfigStorageEmbedded (cmd 1288, 11.0.0+).
pub(crate) fn set_button_config_storage_embedded(
    service: &Session,
    index: i32,
    config: &HidcfgButtonConfigEmbedded,
    name: &HidcfgStorageName,
) -> Result<(), DispatchError> {
    dispatch_in_u32_in_buf_fixed_in_ptr_fixed(
        service,
        index as u32,
        config,
        name,
        proto::SET_BUTTON_CONFIG_STORAGE_EMBEDDED,
    )
}

/// SetButtonConfigStorageFull (cmd 1289, 11.0.0+).
pub(crate) fn set_button_config_storage_full(
    service: &Session,
    index: i32,
    config: &HidcfgButtonConfigFull,
    name: &HidcfgStorageName,
) -> Result<(), DispatchError> {
    dispatch_in_u32_in_buf_fixed_in_ptr_fixed(
        service,
        index as u32,
        config,
        name,
        proto::SET_BUTTON_CONFIG_STORAGE_FULL,
    )
}

/// SetButtonConfigStorageLeft (cmd 1290, 11.0.0+).
pub(crate) fn set_button_config_storage_left(
    service: &Session,
    index: i32,
    config: &HidcfgButtonConfigLeft,
    name: &HidcfgStorageName,
) -> Result<(), DispatchError> {
    dispatch_in_u32_in_buf_fixed_in_ptr_fixed(
        service,
        index as u32,
        config,
        name,
        proto::SET_BUTTON_CONFIG_STORAGE_LEFT,
    )
}

/// SetButtonConfigStorageRight (cmd 1291, 11.0.0+).
pub(crate) fn set_button_config_storage_right(
    service: &Session,
    index: i32,
    config: &HidcfgButtonConfigRight,
    name: &HidcfgStorageName,
) -> Result<(), DispatchError> {
    dispatch_in_u32_in_buf_fixed_in_ptr_fixed(
        service,
        index as u32,
        config,
        name,
        proto::SET_BUTTON_CONFIG_STORAGE_RIGHT,
    )
}

// ---------------------------------------------------------------------------
// Shared dispatch helpers
// ---------------------------------------------------------------------------

fn dispatch_in_buf_out_bool<T>(
    service: &Session,
    buf: &T,
    cmd_id: u32,
) -> Result<bool, DispatchError> {
    // SAFETY: `buf` is a valid reference; viewing it as bytes is sound.
    let buf_bytes =
        unsafe { core::slice::from_raw_parts((buf as *const T).cast::<u8>(), size_of::<T>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(cmd_id)
        .in_buffer(buf_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u8>())
        .send(&mut ipc_buf)?;
    // SAFETY: response payload is at least 1 byte.
    let out = unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    Ok(out & 1 != 0)
}

fn dispatch_in_u32_out_bool(
    service: &Session,
    inval: u32,
    cmd_id: u32,
) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_in_out(service, cmd_id, &inval)?;
    Ok(out & 1 != 0)
}

fn dispatch_in_u64_in_buf_fixed<T>(
    service: &Session,
    inval: u64,
    buf: &T,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `inval` is a `Copy` value on the stack, valid until `.send()`.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const inval).cast::<u8>(), size_of::<u64>()) };
    // SAFETY: `buf` is a valid reference; viewing it as bytes is sound.
    let buf_bytes =
        unsafe { core::slice::from_raw_parts((buf as *const T).cast::<u8>(), size_of::<T>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .in_buffer(
            buf_bytes,
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

fn dispatch_in_u64_out_buf_fixed<T>(
    service: &Session,
    inval: u64,
    buf: &mut T,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `inval` is a `Copy` value on the stack, valid until `.send()`.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const inval).cast::<u8>(), size_of::<u64>()) };
    // SAFETY: `buf` is a valid mutable reference; viewing it as mutable bytes is sound.
    let buf_bytes =
        unsafe { core::slice::from_raw_parts_mut((buf as *mut T).cast::<u8>(), size_of::<T>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_buffer(
            buf_bytes,
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

fn dispatch_in_u32_in_buf_fixed<T>(
    service: &Session,
    inval: u32,
    buf: &T,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `inval` is a `Copy` value on the stack, valid until `.send()`.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const inval).cast::<u8>(), size_of::<u32>()) };
    // SAFETY: `buf` is a valid reference; viewing it as bytes is sound.
    let buf_bytes =
        unsafe { core::slice::from_raw_parts((buf as *const T).cast::<u8>(), size_of::<T>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .in_buffer(
            buf_bytes,
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

fn dispatch_in_u32_out_buf_fixed<T>(
    service: &Session,
    inval: u32,
    buf: &mut T,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `inval` is a `Copy` value on the stack, valid until `.send()`.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const inval).cast::<u8>(), size_of::<u32>()) };
    // SAFETY: `buf` is a valid mutable reference; viewing it as mutable bytes is sound.
    let buf_bytes =
        unsafe { core::slice::from_raw_parts_mut((buf as *mut T).cast::<u8>(), size_of::<T>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_buffer(
            buf_bytes,
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

fn dispatch_in_u32_out_buf_fixed_out_ptr_fixed<T, U>(
    service: &Session,
    inval: u32,
    buf0: &mut T,
    buf1: &mut U,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `inval` is a `Copy` value on the stack, valid until `.send()`.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const inval).cast::<u8>(), size_of::<u32>()) };
    // SAFETY: `buf0` and `buf1` are valid mutable references.
    let buf0_bytes =
        unsafe { core::slice::from_raw_parts_mut((buf0 as *mut T).cast::<u8>(), size_of::<T>()) };
    let buf1_bytes =
        unsafe { core::slice::from_raw_parts_mut((buf1 as *mut U).cast::<u8>(), size_of::<U>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_buffer(
            buf0_bytes,
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::FIXED_SIZE),
        )
        .out_buffer(
            buf1_bytes,
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

fn dispatch_in_u32_in_buf_fixed_in_ptr_fixed<T, U>(
    service: &Session,
    inval: u32,
    buf0: &T,
    buf1: &U,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `inval` is a `Copy` value on the stack, valid until `.send()`.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const inval).cast::<u8>(), size_of::<u32>()) };
    // SAFETY: `buf0` and `buf1` are valid references.
    let buf0_bytes =
        unsafe { core::slice::from_raw_parts((buf0 as *const T).cast::<u8>(), size_of::<T>()) };
    let buf1_bytes =
        unsafe { core::slice::from_raw_parts((buf1 as *const U).cast::<u8>(), size_of::<U>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .in_buffer(
            buf0_bytes,
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::FIXED_SIZE),
        )
        .in_buffer(
            buf1_bytes,
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}
