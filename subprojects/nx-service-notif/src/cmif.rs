//! CMIF protocol operations for the notification service.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Domain,
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::dispatch_in,
    proto,
    types::AlarmSetting,
};

/// Initializes the Application variant (cmd 1000). Sends PID.
pub(crate) fn initialize(domain: &Domain) -> Result<(), DispatchError> {
    let pid_reserved: u64 = 0;
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::INITIALIZE)
        .in_raw(pid_reserved.as_bytes())
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
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::REGISTER_ALARM_SETTING)
        .in_buffer(alarm_setting.as_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(app_param, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u16>())
        .send(&mut buf)?;

    Ok(*result.value::<u16>())
}

/// Updates an existing alarm setting (cmd 510).
///
/// Takes the alarm setting and an optional application parameter buffer.
pub(crate) fn update_alarm_setting(
    domain: &Domain,
    alarm_setting: &AlarmSetting,
    app_param: &[u8],
) -> Result<(), DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::UPDATE_ALARM_SETTING)
        .in_buffer(alarm_setting.as_bytes(), BufferAttr::HIPC_MAP_ALIAS)
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
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::LIST_ALARM_SETTINGS)
        .out_buffer(out.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<i32>())
        .send(&mut buf)?;

    Ok(*result.value::<i32>())
}

/// Loads the application parameter for a given alarm setting (cmd 530).
///
/// Returns the actual number of bytes written to the output buffer.
pub(crate) fn load_application_parameter(
    domain: &Domain,
    alarm_setting_id: u16,
    out: &mut [u8],
) -> Result<u32, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = domain
        .dispatch(proto::LOAD_APPLICATION_PARAMETER)
        .in_raw(alarm_setting_id.as_bytes())
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u32>())
        .send(&mut buf)?;

    Ok(*result.value::<u32>())
}

/// Deletes an alarm setting by ID (cmd 540).
pub(crate) fn delete_alarm_setting(
    domain: &Domain,
    alarm_setting_id: u16,
) -> Result<(), DispatchError> {
    dispatch_in(domain, proto::DELETE_ALARM_SETTING, &alarm_setting_id)
}
