//! CMIF protocol operations for the notification service.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Domain};

use crate::{dispatch::dispatch_in, proto, types::AlarmSetting};

/// Initializes the Application variant (cmd 1000). Sends PID.
pub(crate) fn initialize(domain: &Domain) -> Result<(), DispatchError> {
    let pid_reserved: u64 = 0;
    // SAFETY: `pid_reserved` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const pid_reserved).cast::<u8>(), size_of::<u64>())
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(proto::INITIALIZE)
        .in_raw(in_bytes)
        .send_pid()
        .send(&mut buf)
        .map(|_| ())
}

/// Registers an alarm setting (cmd 500).
///
/// Takes the alarm setting and an optional application parameter buffer.
/// Returns the assigned alarm setting ID.
pub(crate) fn register_alarm_setting(
    domain: &Domain,
    alarm_setting: &AlarmSetting,
    app_param: &[u8],
) -> Result<u16, DispatchError> {
    // SAFETY: `alarm_setting` lives on the caller's stack, valid until `.send()`
    // returns; viewing it as a byte slice for the IN buffer is sound.
    let setting_bytes = unsafe {
        core::slice::from_raw_parts(
            (alarm_setting as *const AlarmSetting).cast::<u8>(),
            size_of::<AlarmSetting>(),
        )
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = domain
        .dispatch(proto::REGISTER_ALARM_SETTING)
        .in_buffer(setting_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(app_param, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u16>())
        .send(&mut buf)?;

    // SAFETY: response payload contains the u16 alarm_setting_id.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<u16>()) })
}

/// Updates an existing alarm setting (cmd 510).
///
/// Takes the alarm setting and an optional application parameter buffer.
pub(crate) fn update_alarm_setting(
    domain: &Domain,
    alarm_setting: &AlarmSetting,
    app_param: &[u8],
) -> Result<(), DispatchError> {
    // SAFETY: `alarm_setting` lives on the caller's stack, valid until `.send()`
    // returns; viewing it as a byte slice for the IN buffer is sound.
    let setting_bytes = unsafe {
        core::slice::from_raw_parts(
            (alarm_setting as *const AlarmSetting).cast::<u8>(),
            size_of::<AlarmSetting>(),
        )
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(proto::UPDATE_ALARM_SETTING)
        .in_buffer(setting_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(app_param, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut buf)
        .map(|_| ())
}

/// Lists all registered alarm settings (cmd 520).
///
/// Writes into the provided buffer and returns the number of entries written.
pub(crate) fn list_alarm_settings(
    domain: &Domain,
    out: &mut [AlarmSetting],
) -> Result<i32, DispatchError> {
    // SAFETY: `out` is a valid `&mut [AlarmSetting]`; reinterpreting its memory
    // as a mutable byte slice for the OUT buffer is sound and the slice lives
    // until `.send()` returns.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(out.as_mut_ptr().cast::<u8>(), core::mem::size_of_val(out))
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = domain
        .dispatch(proto::LIST_ALARM_SETTINGS)
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<i32>())
        .send(&mut buf)?;

    // SAFETY: response payload contains the i32 total count.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

/// Loads the application parameter for a given alarm setting (cmd 530).
///
/// Returns the actual number of bytes written to the output buffer.
pub(crate) fn load_application_parameter(
    domain: &Domain,
    alarm_setting_id: u16,
    out: &mut [u8],
) -> Result<u32, DispatchError> {
    // SAFETY: `alarm_setting_id` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const alarm_setting_id).cast::<u8>(), size_of::<u16>())
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = domain
        .dispatch(proto::LOAD_APPLICATION_PARAMETER)
        .in_raw(in_bytes)
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u32>())
        .send(&mut buf)?;

    // SAFETY: response payload contains the u32 actual size.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}

/// Deletes an alarm setting by ID (cmd 540).
pub(crate) fn delete_alarm_setting(
    domain: &Domain,
    alarm_setting_id: u16,
) -> Result<(), DispatchError> {
    dispatch_in(domain, proto::DELETE_ALARM_SETTING, &alarm_setting_id)
}
