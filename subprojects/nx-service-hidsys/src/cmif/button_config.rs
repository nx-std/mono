//! Button config commands (cmds 1200-1215).
//!
//! Two generations share the same command ID range:
//! - Legacy \[10.0.0-10.2.0\]: keyed by `UniquePadId`, uses opaque `HidsysButtonConfig*` blobs.
//! - v11 \[11.0.0-17.0.1\]: keyed by `BtdrvAddress`, uses typed `HidcfgButtonConfig*` structs.
//!
//! Paired method variants exposed per IC-4 (hosversion-unaware).

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
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
        BtdrvAddress,
        HidcfgButtonConfigEmbedded,
        HidcfgButtonConfigFull,
        HidcfgButtonConfigLeft,
        HidcfgButtonConfigRight,
        HidsysButtonConfigEmbedded,
        HidsysButtonConfigFull,
        HidsysButtonConfigLeft,
        HidsysButtonConfigRight,
        InAddrBoolIn,
        InU64BoolIn,
        UniquePadId,
    },
};

// ---------------------------------------------------------------------------
// Legacy [10.0.0-10.2.0]
// ---------------------------------------------------------------------------

/// LegacyIsButtonConfigSupported (cmd 1200, 10.0.0-10.2.0).
pub(crate) fn legacy_is_button_config_supported(
    service: &Session,
    unique_pad_id: UniquePadId,
) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_in_out(
        service,
        proto::LEGACY_IS_BUTTON_CONFIG_SUPPORTED,
        &unique_pad_id.id,
    )?;
    Ok(out & 1 != 0)
}

/// LegacyDeleteButtonConfig (cmd 1201, 10.0.0-10.2.0).
pub(crate) fn legacy_delete_button_config(
    service: &Session,
    unique_pad_id: UniquePadId,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::LEGACY_DELETE_BUTTON_CONFIG,
        &unique_pad_id.id,
    )
}

/// LegacySetButtonConfigEnabled (cmd 1202, 10.0.0-10.2.0).
pub(crate) fn legacy_set_button_config_enabled(
    service: &Session,
    unique_pad_id: UniquePadId,
    flag: bool,
) -> Result<(), DispatchError> {
    let input = InU64BoolIn {
        flag: flag as u8,
        pad: [0; 7],
        value: unique_pad_id.id,
    };
    dispatch_in(service, proto::LEGACY_SET_BUTTON_CONFIG_ENABLED, &input)
}

/// LegacyIsButtonConfigEnabled (cmd 1203, 10.0.0-10.2.0).
pub(crate) fn legacy_is_button_config_enabled(
    service: &Session,
    unique_pad_id: UniquePadId,
) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_in_out(
        service,
        proto::LEGACY_IS_BUTTON_CONFIG_ENABLED,
        &unique_pad_id.id,
    )?;
    Ok(out & 1 != 0)
}

/// LegacySetButtonConfigEmbedded (cmd 1204, 10.0.0-10.2.0).
pub(crate) fn legacy_set_button_config_embedded(
    service: &Session,
    unique_pad_id: UniquePadId,
    config: &HidsysButtonConfigEmbedded,
) -> Result<(), DispatchError> {
    dispatch_in_u64_in_buf_fixed(
        service,
        unique_pad_id.id,
        config,
        proto::LEGACY_SET_BUTTON_CONFIG_EMBEDDED,
    )
}

/// LegacySetButtonConfigFull (cmd 1205, 10.0.0-10.2.0).
pub(crate) fn legacy_set_button_config_full(
    service: &Session,
    unique_pad_id: UniquePadId,
    config: &HidsysButtonConfigFull,
) -> Result<(), DispatchError> {
    dispatch_in_u64_in_buf_fixed(
        service,
        unique_pad_id.id,
        config,
        proto::LEGACY_SET_BUTTON_CONFIG_FULL,
    )
}

/// LegacySetButtonConfigLeft (cmd 1206, 10.0.0-10.2.0).
pub(crate) fn legacy_set_button_config_left(
    service: &Session,
    unique_pad_id: UniquePadId,
    config: &HidsysButtonConfigLeft,
) -> Result<(), DispatchError> {
    dispatch_in_u64_in_buf_fixed(
        service,
        unique_pad_id.id,
        config,
        proto::LEGACY_SET_BUTTON_CONFIG_LEFT,
    )
}

/// LegacySetButtonConfigRight (cmd 1207, 10.0.0-10.2.0).
pub(crate) fn legacy_set_button_config_right(
    service: &Session,
    unique_pad_id: UniquePadId,
    config: &HidsysButtonConfigRight,
) -> Result<(), DispatchError> {
    dispatch_in_u64_in_buf_fixed(
        service,
        unique_pad_id.id,
        config,
        proto::LEGACY_SET_BUTTON_CONFIG_RIGHT,
    )
}

/// LegacyGetButtonConfigEmbedded (cmd 1208, 10.0.0-10.2.0).
pub(crate) fn legacy_get_button_config_embedded(
    service: &Session,
    unique_pad_id: UniquePadId,
    config: &mut HidsysButtonConfigEmbedded,
) -> Result<(), DispatchError> {
    dispatch_in_u64_out_buf_fixed(
        service,
        unique_pad_id.id,
        config,
        proto::LEGACY_GET_BUTTON_CONFIG_EMBEDDED,
    )
}

/// LegacyGetButtonConfigFull (cmd 1209, 10.0.0-10.2.0).
pub(crate) fn legacy_get_button_config_full(
    service: &Session,
    unique_pad_id: UniquePadId,
    config: &mut HidsysButtonConfigFull,
) -> Result<(), DispatchError> {
    dispatch_in_u64_out_buf_fixed(
        service,
        unique_pad_id.id,
        config,
        proto::LEGACY_GET_BUTTON_CONFIG_FULL,
    )
}

/// LegacyGetButtonConfigLeft (cmd 1210, 10.0.0-10.2.0).
pub(crate) fn legacy_get_button_config_left(
    service: &Session,
    unique_pad_id: UniquePadId,
    config: &mut HidsysButtonConfigLeft,
) -> Result<(), DispatchError> {
    dispatch_in_u64_out_buf_fixed(
        service,
        unique_pad_id.id,
        config,
        proto::LEGACY_GET_BUTTON_CONFIG_LEFT,
    )
}

/// LegacyGetButtonConfigRight (cmd 1211, 10.0.0-10.2.0).
pub(crate) fn legacy_get_button_config_right(
    service: &Session,
    unique_pad_id: UniquePadId,
    config: &mut HidsysButtonConfigRight,
) -> Result<(), DispatchError> {
    dispatch_in_u64_out_buf_fixed(
        service,
        unique_pad_id.id,
        config,
        proto::LEGACY_GET_BUTTON_CONFIG_RIGHT,
    )
}

// ---------------------------------------------------------------------------
// v11 [11.0.0-17.0.1]
// ---------------------------------------------------------------------------

/// IsButtonConfigSupported (cmd 1200, 11.0.0-17.0.1).
pub(crate) fn is_button_config_supported(
    service: &Session,
    addr: BtdrvAddress,
) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_in_out(service, proto::IS_BUTTON_CONFIG_SUPPORTED, &addr)?;
    Ok(out & 1 != 0)
}

/// IsButtonConfigEmbeddedSupported (cmd 1201, 11.0.0-17.0.1).
pub(crate) fn is_button_config_embedded_supported(
    service: &Session,
) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_out(service, proto::IS_BUTTON_CONFIG_EMBEDDED_SUPPORTED)?;
    Ok(out & 1 != 0)
}

/// DeleteButtonConfig (cmd 1202, 11.0.0-17.0.1).
pub(crate) fn delete_button_config(
    service: &Session,
    addr: BtdrvAddress,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::DELETE_BUTTON_CONFIG, &addr)
}

/// DeleteButtonConfigEmbedded (cmd 1203, 11.0.0-17.0.1).
pub(crate) fn delete_button_config_embedded(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::DELETE_BUTTON_CONFIG_EMBEDDED)
}

/// SetButtonConfigEnabled (cmd 1204, 11.0.0-17.0.1).
pub(crate) fn set_button_config_enabled(
    service: &Session,
    addr: BtdrvAddress,
    flag: bool,
) -> Result<(), DispatchError> {
    let input = InAddrBoolIn {
        flag: flag as u8,
        addr,
    };
    dispatch_in(service, proto::SET_BUTTON_CONFIG_ENABLED, &input)
}

/// SetButtonConfigEmbeddedEnabled (cmd 1205, 11.0.0-17.0.1).
pub(crate) fn set_button_config_embedded_enabled(
    service: &Session,
    flag: bool,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::SET_BUTTON_CONFIG_EMBEDDED_ENABLED,
        &(flag as u8),
    )
}

/// IsButtonConfigEnabled (cmd 1206, 11.0.0-17.0.1).
pub(crate) fn is_button_config_enabled(
    service: &Session,
    addr: BtdrvAddress,
) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_in_out(service, proto::IS_BUTTON_CONFIG_ENABLED, &addr)?;
    Ok(out & 1 != 0)
}

/// IsButtonConfigEmbeddedEnabled (cmd 1207, 11.0.0-17.0.1).
pub(crate) fn is_button_config_embedded_enabled(service: &Session) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_out(service, proto::IS_BUTTON_CONFIG_EMBEDDED_ENABLED)?;
    Ok(out & 1 != 0)
}

/// SetButtonConfigEmbedded (cmd 1208, 11.0.0-17.0.1).
pub(crate) fn set_button_config_embedded(
    service: &Session,
    config: &HidcfgButtonConfigEmbedded,
) -> Result<(), DispatchError> {
    dispatch_in_buf_fixed(service, config, proto::SET_BUTTON_CONFIG_EMBEDDED)
}

/// SetButtonConfigFull (cmd 1209, 11.0.0-17.0.1).
pub(crate) fn set_button_config_full(
    service: &Session,
    addr: BtdrvAddress,
    config: &HidcfgButtonConfigFull,
) -> Result<(), DispatchError> {
    dispatch_in_addr_in_buf_fixed(service, addr, config, proto::SET_BUTTON_CONFIG_FULL)
}

/// SetButtonConfigLeft (cmd 1210, 11.0.0-17.0.1).
pub(crate) fn set_button_config_left(
    service: &Session,
    addr: BtdrvAddress,
    config: &HidcfgButtonConfigLeft,
) -> Result<(), DispatchError> {
    dispatch_in_addr_in_buf_fixed(service, addr, config, proto::SET_BUTTON_CONFIG_LEFT)
}

/// SetButtonConfigRight (cmd 1211, 11.0.0-17.0.1).
pub(crate) fn set_button_config_right(
    service: &Session,
    addr: BtdrvAddress,
    config: &HidcfgButtonConfigRight,
) -> Result<(), DispatchError> {
    dispatch_in_addr_in_buf_fixed(service, addr, config, proto::SET_BUTTON_CONFIG_RIGHT)
}

/// GetButtonConfigEmbedded (cmd 1212, 11.0.0-17.0.1).
pub(crate) fn get_button_config_embedded(
    service: &Session,
    config: &mut HidcfgButtonConfigEmbedded,
) -> Result<(), DispatchError> {
    dispatch_out_buf_fixed(service, config, proto::GET_BUTTON_CONFIG_EMBEDDED)
}

/// GetButtonConfigFull (cmd 1213, 11.0.0-17.0.1).
pub(crate) fn get_button_config_full(
    service: &Session,
    addr: BtdrvAddress,
    config: &mut HidcfgButtonConfigFull,
) -> Result<(), DispatchError> {
    dispatch_in_addr_out_buf_fixed(service, addr, config, proto::GET_BUTTON_CONFIG_FULL)
}

/// GetButtonConfigLeft (cmd 1214, 11.0.0-17.0.1).
pub(crate) fn get_button_config_left(
    service: &Session,
    addr: BtdrvAddress,
    config: &mut HidcfgButtonConfigLeft,
) -> Result<(), DispatchError> {
    dispatch_in_addr_out_buf_fixed(service, addr, config, proto::GET_BUTTON_CONFIG_LEFT)
}

/// GetButtonConfigRight (cmd 1215, 11.0.0-17.0.1).
pub(crate) fn get_button_config_right(
    service: &Session,
    addr: BtdrvAddress,
    config: &mut HidcfgButtonConfigRight,
) -> Result<(), DispatchError> {
    dispatch_in_addr_out_buf_fixed(service, addr, config, proto::GET_BUTTON_CONFIG_RIGHT)
}

// ---------------------------------------------------------------------------
// Shared dispatch helpers
// ---------------------------------------------------------------------------

fn dispatch_in_u64_in_buf_fixed<T>(
    service: &Session,
    inval: u64,
    buf: &T,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `inval` is a `Copy` value on the stack, valid until `.send()`.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const inval).cast::<u8>(), size_of::<u64>()) };
    // SAFETY: `buf` is a valid reference; viewing it as bytes for the IN
    // buffer is sound and the slice borrows `buf`.
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
    // SAFETY: `buf` is a valid mutable reference; viewing it as mutable bytes
    // for the OUT buffer is sound.
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

fn dispatch_in_buf_fixed<T>(service: &Session, buf: &T, cmd_id: u32) -> Result<(), DispatchError> {
    // SAFETY: `buf` is a valid reference; viewing it as bytes for the IN
    // buffer is sound.
    let buf_bytes =
        unsafe { core::slice::from_raw_parts((buf as *const T).cast::<u8>(), size_of::<T>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(cmd_id)
        .in_buffer(
            buf_bytes,
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

fn dispatch_out_buf_fixed<T>(
    service: &Session,
    buf: &mut T,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `buf` is a valid mutable reference; viewing it as mutable bytes
    // for the OUT buffer is sound.
    let buf_bytes =
        unsafe { core::slice::from_raw_parts_mut((buf as *mut T).cast::<u8>(), size_of::<T>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(cmd_id)
        .out_buffer(
            buf_bytes,
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

fn dispatch_in_addr_in_buf_fixed<T>(
    service: &Session,
    addr: BtdrvAddress,
    buf: &T,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `addr` is a `Copy` value on the stack, valid until `.send()`.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const addr).cast::<u8>(), size_of::<BtdrvAddress>())
    };
    // SAFETY: `buf` is a valid reference; viewing it as bytes for the IN
    // buffer is sound.
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

fn dispatch_in_addr_out_buf_fixed<T>(
    service: &Session,
    addr: BtdrvAddress,
    buf: &mut T,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `addr` is a `Copy` value on the stack, valid until `.send()`.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const addr).cast::<u8>(), size_of::<BtdrvAddress>())
    };
    // SAFETY: `buf` is a valid mutable reference; viewing it as mutable bytes
    // for the OUT buffer is sound.
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
