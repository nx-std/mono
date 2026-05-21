//! CMIF protocol operations for the power supply monitor service.

use core::{mem::size_of, ptr};

use nx_sf::{
    cmif,
    ipc::{self, Handle as SessionHandle},
};

use crate::{
    proto,
    types::{BatteryChargeInfoFields, BatteryChargeInfoFieldsLegacy},
};

// ---------------------------------------------------------------------------
// Dispatch helpers
// ---------------------------------------------------------------------------

fn dispatch_no_io(session: SessionHandle, cmd_id: u32) -> Result<(), DispatchNoIoError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifRequestBuilder::new(cmd_id)
            .send(&mut buf)
            .map_err(DispatchNoIoError::BuildRequest)?;
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(DispatchNoIoError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(&buf, 0).map_err(DispatchNoIoError::ParseResponse)?;

    Ok(())
}

/// Error returned by no-IO dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchNoIoError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

fn dispatch_out_u32(session: SessionHandle, cmd_id: u32) -> Result<u32, DispatchOutU32Error> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifRequestBuilder::new(cmd_id)
            .send(&mut buf)
            .map_err(DispatchOutU32Error::BuildRequest)?;
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(DispatchOutU32Error::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, size_of::<u32>())
        .map_err(DispatchOutU32Error::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u32.
    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(value)
}

/// Error returned by out-u32 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchOutU32Error {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

fn dispatch_out_f64(session: SessionHandle, cmd_id: u32) -> Result<f64, DispatchOutF64Error> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifRequestBuilder::new(cmd_id)
            .send(&mut buf)
            .map_err(DispatchOutF64Error::BuildRequest)?;
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(DispatchOutF64Error::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, size_of::<f64>())
        .map_err(DispatchOutF64Error::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for f64.
    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<f64>()) };

    Ok(value)
}

/// Error returned by out-f64 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchOutF64Error {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

fn dispatch_out_bool(session: SessionHandle, cmd_id: u32) -> Result<bool, DispatchOutBoolError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifRequestBuilder::new(cmd_id)
            .send(&mut buf)
            .map_err(DispatchOutBoolError::BuildRequest)?;
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(DispatchOutBoolError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, size_of::<u8>())
        .map_err(DispatchOutBoolError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u8.
    let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(raw & 1 != 0)
}

/// Error returned by out-bool dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchOutBoolError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

fn dispatch_in_bool(
    session: SessionHandle,
    cmd_id: u32,
    value: bool,
) -> Result<(), DispatchInBoolError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmd_id)
            .data_size(size_of::<u8>())
            .send(&mut buf)
            .map_err(DispatchInBoolError::BuildRequest)?;

        // SAFETY: `req` is exactly `size_of::<u8>()` bytes.
        unsafe { ptr::write_unaligned(req.as_mut_ptr().cast::<u8>(), value as u8) };
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(DispatchInBoolError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(&buf, 0).map_err(DispatchInBoolError::ParseResponse)?;

    Ok(())
}

/// Error returned by in-bool dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchInBoolError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

fn dispatch_event(session: SessionHandle, cmd_id: u32) -> Result<u32, DispatchEventError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifRequestBuilder::new(cmd_id)
            .send(&mut buf)
            .map_err(DispatchEventError::BuildRequest)?;
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(DispatchEventError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, 0).map_err(DispatchEventError::ParseResponse)?;

    if resp.copy_handles.is_empty() {
        return Err(DispatchEventError::MissingHandle);
    }

    Ok(resp.copy_handles[0])
}

/// Error returned by event acquisition dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchEventError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    #[error("response did not contain expected copy handle")]
    MissingHandle,
}

fn dispatch_out_struct<T: Copy>(
    session: SessionHandle,
    cmd_id: u32,
) -> Result<T, DispatchOutStructError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifRequestBuilder::new(cmd_id)
            .send(&mut buf)
            .map_err(DispatchOutStructError::BuildRequest)?;
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(DispatchOutStructError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, size_of::<T>())
        .map_err(DispatchOutStructError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for T.
    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<T>()) };

    Ok(value)
}

/// Error returned by out-struct dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchOutStructError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

fn dispatch_open_session(session: SessionHandle) -> Result<SessionHandle, OpenSessionError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifRequestBuilder::new(proto::OPEN_SESSION)
            .send(&mut buf)
            .map_err(OpenSessionError::BuildRequest)?;
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(OpenSessionError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, 0).map_err(OpenSessionError::ParseResponse)?;

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
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
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
