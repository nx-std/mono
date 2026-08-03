//! CMIF protocol operations for the backlight service.

use nx_sf::{cmif, service::BorrowedSessionHandle};

use crate::proto;

fn dispatch_no_io(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
) -> Result<(), DispatchNoIoError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmd_id).build();
    req.send(&mut buf, session)
        .map_err(DispatchNoIoError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DispatchNoIoError::ParseResponse)?;

    Ok(())
}

/// Error returned by no-IO dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchNoIoError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

fn dispatch_in_u64(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    value: u64,
) -> Result<(), DispatchInU64Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .with_data_value(&value)
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchInU64Error::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DispatchInU64Error::ParseResponse)?;

    Ok(())
}

/// Error returned by in-u64 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchInU64Error {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

fn dispatch_in_f32(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    value: f32,
) -> Result<(), DispatchInF32Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .with_data_value(&value)
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchInF32Error::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DispatchInF32Error::ParseResponse)?;

    Ok(())
}

/// Error returned by in-f32 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchInF32Error {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

fn dispatch_out_f32(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
) -> Result<f32, DispatchOutF32Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmd_id).build();
    req.send(&mut buf, session)
        .map_err(DispatchOutF32Error::SendRequest)?;

    let resp = cmif::parse_response::<&f32>(&buf).map_err(DispatchOutF32Error::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by out-f32 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchOutF32Error {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

fn dispatch_out_bool(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
) -> Result<bool, DispatchOutBoolError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmd_id).build();
    req.send(&mut buf, session)
        .map_err(DispatchOutBoolError::SendRequest)?;

    let resp = cmif::parse_response::<&u8>(&buf).map_err(DispatchOutBoolError::ParseResponse)?;

    Ok(*resp.payload & 1 != 0)
}

/// Error returned by out-bool dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchOutBoolError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

fn dispatch_out_u32(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
) -> Result<u32, DispatchOutU32Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmd_id).build();
    req.send(&mut buf, session)
        .map_err(DispatchOutU32Error::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(DispatchOutU32Error::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by out-u32 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchOutU32Error {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

pub fn save_current_setting(session: BorrowedSessionHandle<'_>) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::SAVE_CURRENT_SETTING)
}

pub fn load_current_setting(session: BorrowedSessionHandle<'_>) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::LOAD_CURRENT_SETTING)
}

pub fn set_current_brightness_setting(
    session: BorrowedSessionHandle<'_>,
    brightness: f32,
) -> Result<(), DispatchInF32Error> {
    dispatch_in_f32(session, proto::SET_CURRENT_BRIGHTNESS_SETTING, brightness)
}

pub fn get_current_brightness_setting(
    session: BorrowedSessionHandle<'_>,
) -> Result<f32, DispatchOutF32Error> {
    dispatch_out_f32(session, proto::GET_CURRENT_BRIGHTNESS_SETTING)
}

pub fn apply_current_brightness_setting_to_backlight(
    session: BorrowedSessionHandle<'_>,
) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(
        session,
        proto::APPLY_CURRENT_BRIGHTNESS_SETTING_TO_BACKLIGHT,
    )
}

pub fn get_brightness_setting_applied_to_backlight(
    session: BorrowedSessionHandle<'_>,
) -> Result<f32, DispatchOutF32Error> {
    dispatch_out_f32(session, proto::GET_BRIGHTNESS_SETTING_APPLIED_TO_BACKLIGHT)
}

pub fn switch_backlight_on(
    session: BorrowedSessionHandle<'_>,
    fade_time: u64,
) -> Result<(), DispatchInU64Error> {
    dispatch_in_u64(session, proto::SWITCH_BACKLIGHT_ON, fade_time)
}

pub fn switch_backlight_off(
    session: BorrowedSessionHandle<'_>,
    fade_time: u64,
) -> Result<(), DispatchInU64Error> {
    dispatch_in_u64(session, proto::SWITCH_BACKLIGHT_OFF, fade_time)
}

pub fn get_backlight_switch_status(
    session: BorrowedSessionHandle<'_>,
) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::GET_BACKLIGHT_SWITCH_STATUS)
}

pub fn enable_dimming(session: BorrowedSessionHandle<'_>) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::ENABLE_DIMMING)
}

pub fn disable_dimming(session: BorrowedSessionHandle<'_>) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::DISABLE_DIMMING)
}

pub fn is_dimming_enabled(
    session: BorrowedSessionHandle<'_>,
) -> Result<bool, DispatchOutBoolError> {
    dispatch_out_bool(session, proto::IS_DIMMING_ENABLED)
}

pub fn enable_auto_brightness_control(
    session: BorrowedSessionHandle<'_>,
) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::ENABLE_AUTO_BRIGHTNESS_CONTROL)
}

pub fn disable_auto_brightness_control(
    session: BorrowedSessionHandle<'_>,
) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::DISABLE_AUTO_BRIGHTNESS_CONTROL)
}

pub fn is_auto_brightness_control_enabled(
    session: BorrowedSessionHandle<'_>,
) -> Result<bool, DispatchOutBoolError> {
    dispatch_out_bool(session, proto::IS_AUTO_BRIGHTNESS_CONTROL_ENABLED)
}

pub fn set_ambient_light_sensor_value(
    session: BorrowedSessionHandle<'_>,
    value: f32,
) -> Result<(), DispatchInF32Error> {
    dispatch_in_f32(session, proto::SET_AMBIENT_LIGHT_SENSOR_VALUE, value)
}

pub fn get_ambient_light_sensor_value(
    session: BorrowedSessionHandle<'_>,
) -> Result<GetAmbientLightSensorValueOut, GetAmbientLightSensorValueError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_AMBIENT_LIGHT_SENSOR_VALUE).build();
    req.send(&mut buf, session)
        .map_err(GetAmbientLightSensorValueError::SendRequest)?;

    // Wire layout: { u32 over_limit, f32 lux } = 8 bytes.
    let resp = cmif::parse_response::<&AmbientLightSensorRaw>(&buf)
        .map_err(GetAmbientLightSensorValueError::ParseResponse)?;

    Ok(GetAmbientLightSensorValueOut {
        over_limit: resp.payload.over_limit & 1 != 0,
        lux: resp.payload.lux,
    })
}

/// Wire layout for [`get_ambient_light_sensor_value`].
#[repr(C)]
#[derive(zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
struct AmbientLightSensorRaw {
    over_limit: u32,
    lux: f32,
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
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

pub fn is_ambient_light_sensor_available(
    session: BorrowedSessionHandle<'_>,
) -> Result<bool, DispatchOutBoolError> {
    dispatch_out_bool(session, proto::IS_AMBIENT_LIGHT_SENSOR_AVAILABLE)
}

pub fn set_current_brightness_setting_for_vr_mode(
    session: BorrowedSessionHandle<'_>,
    brightness: f32,
) -> Result<(), DispatchInF32Error> {
    dispatch_in_f32(
        session,
        proto::SET_CURRENT_BRIGHTNESS_SETTING_FOR_VR_MODE,
        brightness,
    )
}

pub fn get_current_brightness_setting_for_vr_mode(
    session: BorrowedSessionHandle<'_>,
) -> Result<f32, DispatchOutF32Error> {
    dispatch_out_f32(session, proto::GET_CURRENT_BRIGHTNESS_SETTING_FOR_VR_MODE)
}

pub fn enable_vr_mode(session: BorrowedSessionHandle<'_>) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::ENABLE_VR_MODE)
}

pub fn disable_vr_mode(session: BorrowedSessionHandle<'_>) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::DISABLE_VR_MODE)
}

pub fn is_vr_mode_enabled(
    session: BorrowedSessionHandle<'_>,
) -> Result<bool, DispatchOutBoolError> {
    dispatch_out_bool(session, proto::IS_VR_MODE_ENABLED)
}
