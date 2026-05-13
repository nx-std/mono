//! CMIF protocol operations for the power supply monitor service.

use core::ptr;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::{
    proto,
    types::{BatteryChargeInfoFields, BatteryChargeInfoFieldsLegacy},
};

// ---------------------------------------------------------------------------
// Dispatch helpers
// ---------------------------------------------------------------------------

fn dispatch_no_io(session: SessionHandle, cmd_id: u32) -> Result<(), DispatchNoIoError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(DispatchNoIoError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(DispatchNoIoError::ParseResponse)?;

    Ok(())
}

/// Error returned by no-IO dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchNoIoError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

fn dispatch_out_u32(session: SessionHandle, cmd_id: u32) -> Result<u32, DispatchOutU32Error> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(DispatchOutU32Error::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u32>()) }
        .map_err(DispatchOutU32Error::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u32.
    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(value)
}

/// Error returned by out-u32 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchOutU32Error {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

fn dispatch_out_f64(session: SessionHandle, cmd_id: u32) -> Result<f64, DispatchOutF64Error> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(DispatchOutF64Error::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<f64>()) }
        .map_err(DispatchOutF64Error::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for f64.
    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<f64>()) };

    Ok(value)
}

/// Error returned by out-f64 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchOutF64Error {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

fn dispatch_out_bool(session: SessionHandle, cmd_id: u32) -> Result<bool, DispatchOutBoolError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(DispatchOutBoolError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u8>()) }
        .map_err(DispatchOutBoolError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u8.
    let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(raw & 1 != 0)
}

/// Error returned by out-bool dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchOutBoolError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

fn dispatch_in_bool(
    session: SessionHandle,
    cmd_id: u32,
    value: bool,
) -> Result<(), DispatchInBoolError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id)
        .data_size(size_of::<u8>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u8.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u8>().cast_mut(), value as u8);
    }

    ipc::send_sync_request(session).map_err(DispatchInBoolError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(DispatchInBoolError::ParseResponse)?;

    Ok(())
}

/// Error returned by in-bool dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchInBoolError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

fn dispatch_event(session: SessionHandle, cmd_id: u32) -> Result<u32, DispatchEventError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(DispatchEventError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(DispatchEventError::ParseResponse)?;

    if resp.copy_handles.is_empty() {
        return Err(DispatchEventError::MissingHandle);
    }

    Ok(resp.copy_handles[0])
}

/// Error returned by event acquisition dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchEventError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    #[error("response did not contain expected copy handle")]
    MissingHandle,
}

fn dispatch_out_struct<T: Copy>(
    session: SessionHandle,
    cmd_id: u32,
) -> Result<T, DispatchOutStructError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(DispatchOutStructError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<T>()) }
        .map_err(DispatchOutStructError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for T.
    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<T>()) };

    Ok(value)
}

/// Error returned by out-struct dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchOutStructError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

fn dispatch_open_session(session: SessionHandle) -> Result<SessionHandle, OpenSessionError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::OPEN_SESSION).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(OpenSessionError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(OpenSessionError::ParseResponse)?;

    let raw_handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(OpenSessionError::MissingHandle)?;

    // SAFETY: the handle comes from a valid IPC response.
    Ok(unsafe { SessionHandle::from_raw(raw_handle) })
}

/// Error returned by [`open_session`].
#[derive(Debug, thiserror::Error)]
pub enum OpenSessionError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    #[error("response did not contain expected move handle")]
    MissingHandle,
}

// ---------------------------------------------------------------------------
// IPsmServer public command functions
// ---------------------------------------------------------------------------

pub fn get_battery_charge_percentage(session: SessionHandle) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::GET_BATTERY_CHARGE_PERCENTAGE)
}

pub fn get_charger_type(session: SessionHandle) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::GET_CHARGER_TYPE)
}

pub fn enable_battery_charging(session: SessionHandle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::ENABLE_BATTERY_CHARGING)
}

pub fn disable_battery_charging(session: SessionHandle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::DISABLE_BATTERY_CHARGING)
}

pub fn is_battery_charging_enabled(session: SessionHandle) -> Result<bool, DispatchOutBoolError> {
    dispatch_out_bool(session, proto::IS_BATTERY_CHARGING_ENABLED)
}

pub fn acquire_controller_power_supply(session: SessionHandle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::ACQUIRE_CONTROLLER_POWER_SUPPLY)
}

pub fn release_controller_power_supply(session: SessionHandle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::RELEASE_CONTROLLER_POWER_SUPPLY)
}

pub fn open_session(session: SessionHandle) -> Result<SessionHandle, OpenSessionError> {
    dispatch_open_session(session)
}

pub fn enable_enough_power_charge_emulation(
    session: SessionHandle,
) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::ENABLE_ENOUGH_POWER_CHARGE_EMULATION)
}

pub fn disable_enough_power_charge_emulation(
    session: SessionHandle,
) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::DISABLE_ENOUGH_POWER_CHARGE_EMULATION)
}

pub fn enable_fast_battery_charging(session: SessionHandle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::ENABLE_FAST_BATTERY_CHARGING)
}

pub fn disable_fast_battery_charging(session: SessionHandle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::DISABLE_FAST_BATTERY_CHARGING)
}

pub fn get_battery_voltage_state(session: SessionHandle) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::GET_BATTERY_VOLTAGE_STATE)
}

pub fn get_raw_battery_charge_percentage(
    session: SessionHandle,
) -> Result<f64, DispatchOutF64Error> {
    dispatch_out_f64(session, proto::GET_RAW_BATTERY_CHARGE_PERCENTAGE)
}

pub fn is_enough_power_supplied(session: SessionHandle) -> Result<bool, DispatchOutBoolError> {
    dispatch_out_bool(session, proto::IS_ENOUGH_POWER_SUPPLIED)
}

pub fn get_battery_age_percentage(session: SessionHandle) -> Result<f64, DispatchOutF64Error> {
    dispatch_out_f64(session, proto::GET_BATTERY_AGE_PERCENTAGE)
}

pub fn get_battery_charge_info_event(session: SessionHandle) -> Result<u32, DispatchEventError> {
    dispatch_event(session, proto::GET_BATTERY_CHARGE_INFO_EVENT)
}

pub fn get_battery_charge_info_fields_legacy(
    session: SessionHandle,
) -> Result<BatteryChargeInfoFieldsLegacy, DispatchOutStructError> {
    dispatch_out_struct(session, proto::GET_BATTERY_CHARGE_INFO_FIELDS)
}

pub fn get_battery_charge_info_fields(
    session: SessionHandle,
) -> Result<BatteryChargeInfoFields, DispatchOutStructError> {
    dispatch_out_struct(session, proto::GET_BATTERY_CHARGE_INFO_FIELDS)
}

pub fn get_battery_charge_calibrated_event(
    session: SessionHandle,
) -> Result<u32, DispatchEventError> {
    dispatch_event(session, proto::GET_BATTERY_CHARGE_CALIBRATED_EVENT)
}

// ---------------------------------------------------------------------------
// IPsmSession public command functions
// ---------------------------------------------------------------------------

pub fn session_bind_state_change_event(session: SessionHandle) -> Result<u32, DispatchEventError> {
    dispatch_event(session, proto::SESSION_BIND_STATE_CHANGE_EVENT)
}

pub fn session_unbind_state_change_event(session: SessionHandle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::SESSION_UNBIND_STATE_CHANGE_EVENT)
}

pub fn session_set_charger_type_change_event_enabled(
    session: SessionHandle,
    enabled: bool,
) -> Result<(), DispatchInBoolError> {
    dispatch_in_bool(
        session,
        proto::SESSION_SET_CHARGER_TYPE_CHANGE_EVENT_ENABLED,
        enabled,
    )
}

pub fn session_set_power_supply_change_event_enabled(
    session: SessionHandle,
    enabled: bool,
) -> Result<(), DispatchInBoolError> {
    dispatch_in_bool(
        session,
        proto::SESSION_SET_POWER_SUPPLY_CHANGE_EVENT_ENABLED,
        enabled,
    )
}

pub fn session_set_battery_voltage_state_change_event_enabled(
    session: SessionHandle,
    enabled: bool,
) -> Result<(), DispatchInBoolError> {
    dispatch_in_bool(
        session,
        proto::SESSION_SET_BATTERY_VOLTAGE_STATE_CHANGE_EVENT_ENABLED,
        enabled,
    )
}
