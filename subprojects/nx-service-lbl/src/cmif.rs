//! CMIF protocol operations for the backlight service.

use core::ptr;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto;

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

fn dispatch_in_u64(
    session: SessionHandle,
    cmd_id: u32,
    value: u64,
) -> Result<(), DispatchInU64Error> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id)
        .data_size(size_of::<u64>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u64.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), value);
    }

    ipc::send_sync_request(session).map_err(DispatchInU64Error::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(DispatchInU64Error::ParseResponse)?;

    Ok(())
}

/// Error returned by in-u64 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchInU64Error {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

fn dispatch_in_f32(
    session: SessionHandle,
    cmd_id: u32,
    value: f32,
) -> Result<(), DispatchInF32Error> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id)
        .data_size(size_of::<f32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for f32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<f32>().cast_mut(), value);
    }

    ipc::send_sync_request(session).map_err(DispatchInF32Error::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(DispatchInF32Error::ParseResponse)?;

    Ok(())
}

/// Error returned by in-f32 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchInF32Error {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

fn dispatch_out_f32(session: SessionHandle, cmd_id: u32) -> Result<f32, DispatchOutF32Error> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(DispatchOutF32Error::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<f32>()) }
        .map_err(DispatchOutF32Error::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for f32.
    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<f32>()) };

    Ok(value)
}

/// Error returned by out-f32 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchOutF32Error {
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

// ---------------------------------------------------------------------------
// Public command functions
// ---------------------------------------------------------------------------

pub fn save_current_setting(session: SessionHandle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::SAVE_CURRENT_SETTING)
}

pub fn load_current_setting(session: SessionHandle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::LOAD_CURRENT_SETTING)
}

pub fn set_current_brightness_setting(
    session: SessionHandle,
    brightness: f32,
) -> Result<(), DispatchInF32Error> {
    dispatch_in_f32(session, proto::SET_CURRENT_BRIGHTNESS_SETTING, brightness)
}

pub fn get_current_brightness_setting(session: SessionHandle) -> Result<f32, DispatchOutF32Error> {
    dispatch_out_f32(session, proto::GET_CURRENT_BRIGHTNESS_SETTING)
}

pub fn apply_current_brightness_setting_to_backlight(
    session: SessionHandle,
) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(
        session,
        proto::APPLY_CURRENT_BRIGHTNESS_SETTING_TO_BACKLIGHT,
    )
}

pub fn get_brightness_setting_applied_to_backlight(
    session: SessionHandle,
) -> Result<f32, DispatchOutF32Error> {
    dispatch_out_f32(session, proto::GET_BRIGHTNESS_SETTING_APPLIED_TO_BACKLIGHT)
}

pub fn switch_backlight_on(
    session: SessionHandle,
    fade_time: u64,
) -> Result<(), DispatchInU64Error> {
    dispatch_in_u64(session, proto::SWITCH_BACKLIGHT_ON, fade_time)
}

pub fn switch_backlight_off(
    session: SessionHandle,
    fade_time: u64,
) -> Result<(), DispatchInU64Error> {
    dispatch_in_u64(session, proto::SWITCH_BACKLIGHT_OFF, fade_time)
}

pub fn get_backlight_switch_status(session: SessionHandle) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::GET_BACKLIGHT_SWITCH_STATUS)
}

pub fn enable_dimming(session: SessionHandle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::ENABLE_DIMMING)
}

pub fn disable_dimming(session: SessionHandle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::DISABLE_DIMMING)
}

pub fn is_dimming_enabled(session: SessionHandle) -> Result<bool, DispatchOutBoolError> {
    dispatch_out_bool(session, proto::IS_DIMMING_ENABLED)
}

pub fn enable_auto_brightness_control(session: SessionHandle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::ENABLE_AUTO_BRIGHTNESS_CONTROL)
}

pub fn disable_auto_brightness_control(session: SessionHandle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::DISABLE_AUTO_BRIGHTNESS_CONTROL)
}

pub fn is_auto_brightness_control_enabled(
    session: SessionHandle,
) -> Result<bool, DispatchOutBoolError> {
    dispatch_out_bool(session, proto::IS_AUTO_BRIGHTNESS_CONTROL_ENABLED)
}

pub fn set_ambient_light_sensor_value(
    session: SessionHandle,
    value: f32,
) -> Result<(), DispatchInF32Error> {
    dispatch_in_f32(session, proto::SET_AMBIENT_LIGHT_SENSOR_VALUE, value)
}

pub fn get_ambient_light_sensor_value(
    session: SessionHandle,
) -> Result<GetAmbientLightSensorValueOut, GetAmbientLightSensorValueError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_AMBIENT_LIGHT_SENSOR_VALUE).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetAmbientLightSensorValueError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    // Wire layout: { u32 over_limit, f32 lux } = 8 bytes.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<[u32; 2]>()) }
        .map_err(GetAmbientLightSensorValueError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for {u32, f32}.
    let (over_limit_raw, lux) = unsafe {
        let data_ptr = resp.data.as_ptr().cast::<u32>();
        let over_limit_raw = ptr::read_unaligned(data_ptr);
        let lux = ptr::read_unaligned(data_ptr.add(1).cast::<f32>());
        (over_limit_raw, lux)
    };

    Ok(GetAmbientLightSensorValueOut {
        over_limit: over_limit_raw & 1 != 0,
        lux,
    })
}

/// Raw output from [`get_ambient_light_sensor_value`].
pub struct GetAmbientLightSensorValueOut {
    pub over_limit: bool,
    pub lux: f32,
}

/// Error returned by [`get_ambient_light_sensor_value`].
#[derive(Debug, thiserror::Error)]
pub enum GetAmbientLightSensorValueError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

pub fn is_ambient_light_sensor_available(
    session: SessionHandle,
) -> Result<bool, DispatchOutBoolError> {
    dispatch_out_bool(session, proto::IS_AMBIENT_LIGHT_SENSOR_AVAILABLE)
}

pub fn set_current_brightness_setting_for_vr_mode(
    session: SessionHandle,
    brightness: f32,
) -> Result<(), DispatchInF32Error> {
    dispatch_in_f32(
        session,
        proto::SET_CURRENT_BRIGHTNESS_SETTING_FOR_VR_MODE,
        brightness,
    )
}

pub fn get_current_brightness_setting_for_vr_mode(
    session: SessionHandle,
) -> Result<f32, DispatchOutF32Error> {
    dispatch_out_f32(session, proto::GET_CURRENT_BRIGHTNESS_SETTING_FOR_VR_MODE)
}

pub fn enable_vr_mode(session: SessionHandle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::ENABLE_VR_MODE)
}

pub fn disable_vr_mode(session: SessionHandle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::DISABLE_VR_MODE)
}

pub fn is_vr_mode_enabled(session: SessionHandle) -> Result<bool, DispatchOutBoolError> {
    dispatch_out_bool(session, proto::IS_VR_MODE_ENABLED)
}
