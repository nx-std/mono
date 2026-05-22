//! CMIF protocol operations for the audio control service.

use core::{mem::size_of, ptr};

use nx_sf::{
    cmif,
    ipc::{self, Handle as SessionHandle},
};

use crate::{
    proto,
    types::{SetDefaultTargetIn, SetTargetMuteIn, SetTargetVolumeIn, TargetModeIn},
};

fn dispatch_no_io(session: SessionHandle, cmd_id: u32) -> Result<(), DispatchNoIoError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(cmd_id).build();
    req.write_to(&mut buf)
        .map_err(DispatchNoIoError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(DispatchNoIoError::SendRequest)?;

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

fn dispatch_in_u32(
    session: SessionHandle,
    cmd_id: u32,
    value: u32,
) -> Result<(), DispatchInU32Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .data_size(size_of::<u32>())
        .build();
    req.write_to(&mut buf)
        .map_err(DispatchInU32Error::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<u32>()` bytes.
    unsafe { ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<u32>(), value) };

    ipc::send_sync_request(&mut buf, session).map_err(DispatchInU32Error::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(DispatchInU32Error::ParseResponse)?;

    Ok(())
}

/// Error returned by in-u32 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchInU32Error {
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
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .data_size(size_of::<bool>())
        .build();
    req.write_to(&mut buf)
        .map_err(DispatchInBoolError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<bool>()` bytes.
    unsafe { ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<bool>(), value) };

    ipc::send_sync_request(&mut buf, session).map_err(DispatchInBoolError::SendRequest)?;

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

fn dispatch_in_f32(
    session: SessionHandle,
    cmd_id: u32,
    value: f32,
) -> Result<(), DispatchInF32Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .data_size(size_of::<f32>())
        .build();
    req.write_to(&mut buf)
        .map_err(DispatchInF32Error::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<f32>()` bytes.
    unsafe { ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<f32>(), value) };

    ipc::send_sync_request(&mut buf, session).map_err(DispatchInF32Error::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(DispatchInF32Error::ParseResponse)?;

    Ok(())
}

/// Error returned by in-f32 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchInF32Error {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

fn dispatch_in_struct<T: Copy>(
    session: SessionHandle,
    cmd_id: u32,
    value: &T,
) -> Result<(), DispatchInStructError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .data_size(size_of::<T>())
        .build();
    req.write_to(&mut buf)
        .map_err(DispatchInStructError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<T>()` bytes.
    unsafe { ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<T>(), *value) };

    ipc::send_sync_request(&mut buf, session).map_err(DispatchInStructError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(DispatchInStructError::ParseResponse)?;

    Ok(())
}

/// Error returned by in-struct dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchInStructError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

fn dispatch_out_i32(session: SessionHandle, cmd_id: u32) -> Result<i32, DispatchOutI32Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(cmd_id).build();
    req.write_to(&mut buf)
        .map_err(DispatchOutI32Error::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(DispatchOutI32Error::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<i32>())
        .map_err(DispatchOutI32Error::ParseResponse)?;

    // SAFETY: resp.data points to at least `size_of::<i32>()` bytes.
    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };

    Ok(value)
}

/// Error returned by out-i32 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchOutI32Error {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

fn dispatch_out_u32(session: SessionHandle, cmd_id: u32) -> Result<u32, DispatchOutU32Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(cmd_id).build();
    req.write_to(&mut buf)
        .map_err(DispatchOutU32Error::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(DispatchOutU32Error::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u32>())
        .map_err(DispatchOutU32Error::ParseResponse)?;

    // SAFETY: resp.data points to at least `size_of::<u32>()` bytes.
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

fn dispatch_out_f32(session: SessionHandle, cmd_id: u32) -> Result<f32, DispatchOutF32Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(cmd_id).build();
    req.write_to(&mut buf)
        .map_err(DispatchOutF32Error::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(DispatchOutF32Error::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<f32>())
        .map_err(DispatchOutF32Error::ParseResponse)?;

    // SAFETY: resp.data points to at least `size_of::<f32>()` bytes.
    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<f32>()) };

    Ok(value)
}

/// Error returned by out-f32 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchOutF32Error {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

fn dispatch_in_u32_out_i32(
    session: SessionHandle,
    cmd_id: u32,
    input: u32,
) -> Result<i32, DispatchInU32OutI32Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .data_size(size_of::<u32>())
        .build();
    req.write_to(&mut buf)
        .map_err(DispatchInU32OutI32Error::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<u32>()` bytes.
    unsafe { ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<u32>(), input) };

    ipc::send_sync_request(&mut buf, session).map_err(DispatchInU32OutI32Error::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<i32>())
        .map_err(DispatchInU32OutI32Error::ParseResponse)?;

    // SAFETY: resp.data points to at least `size_of::<i32>()` bytes.
    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };

    Ok(value)
}

/// Error returned by in-u32/out-i32 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchInU32OutI32Error {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

fn dispatch_in_u32_out_u32(
    session: SessionHandle,
    cmd_id: u32,
    input: u32,
) -> Result<u32, DispatchInU32OutU32Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .data_size(size_of::<u32>())
        .build();
    req.write_to(&mut buf)
        .map_err(DispatchInU32OutU32Error::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<u32>()` bytes.
    unsafe { ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<u32>(), input) };

    ipc::send_sync_request(&mut buf, session).map_err(DispatchInU32OutU32Error::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u32>())
        .map_err(DispatchInU32OutU32Error::ParseResponse)?;

    // SAFETY: resp.data points to at least `size_of::<u32>()` bytes.
    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(value)
}

/// Error returned by in-u32/out-u32 dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchInU32OutU32Error {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

fn dispatch_in_u32_out_bool(
    session: SessionHandle,
    cmd_id: u32,
    input: u32,
) -> Result<bool, DispatchInU32OutBoolError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .data_size(size_of::<u32>())
        .build();
    req.write_to(&mut buf)
        .map_err(DispatchInU32OutBoolError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<u32>()` bytes.
    unsafe { ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<u32>(), input) };

    ipc::send_sync_request(&mut buf, session).map_err(DispatchInU32OutBoolError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u8>())
        .map_err(DispatchInU32OutBoolError::ParseResponse)?;

    // SAFETY: resp.data points to at least `size_of::<u8>()` bytes.
    let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(raw & 1 != 0)
}

/// Error returned by in-u32/out-bool dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchInU32OutBoolError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

fn dispatch_event(session: SessionHandle, cmd_id: u32) -> Result<u32, DispatchEventError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(cmd_id).build();
    req.write_to(&mut buf)
        .map_err(DispatchEventError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(DispatchEventError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, 0).map_err(DispatchEventError::ParseResponse)?;

    let Some(&handle) = resp.copy_handles.first() else {
        return Err(DispatchEventError::MissingHandle);
    };

    Ok(handle)
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

pub fn get_target_volume(
    session: SessionHandle,
    target: u32,
) -> Result<i32, DispatchInU32OutI32Error> {
    dispatch_in_u32_out_i32(session, proto::GET_TARGET_VOLUME, target)
}

pub fn set_target_volume(
    session: SessionHandle,
    target: u32,
    volume: i32,
) -> Result<(), DispatchInStructError> {
    dispatch_in_struct(
        session,
        proto::SET_TARGET_VOLUME,
        &SetTargetVolumeIn { target, volume },
    )
}

pub fn get_target_volume_min(session: SessionHandle) -> Result<i32, DispatchOutI32Error> {
    dispatch_out_i32(session, proto::GET_TARGET_VOLUME_MIN)
}

pub fn get_target_volume_max(session: SessionHandle) -> Result<i32, DispatchOutI32Error> {
    dispatch_out_i32(session, proto::GET_TARGET_VOLUME_MAX)
}

pub fn is_target_mute(
    session: SessionHandle,
    target: u32,
) -> Result<bool, DispatchInU32OutBoolError> {
    dispatch_in_u32_out_bool(session, proto::IS_TARGET_MUTE, target)
}

pub fn set_target_mute(
    session: SessionHandle,
    target: u32,
    mute: bool,
) -> Result<(), DispatchInStructError> {
    dispatch_in_struct(
        session,
        proto::SET_TARGET_MUTE,
        &SetTargetMuteIn {
            mute: mute as u32,
            target,
        },
    )
}

pub fn is_target_connected(
    session: SessionHandle,
    target: u32,
) -> Result<bool, DispatchInU32OutBoolError> {
    dispatch_in_u32_out_bool(session, proto::IS_TARGET_CONNECTED, target)
}

pub fn set_default_target(
    session: SessionHandle,
    target: u32,
    fade_in_ns: u64,
    fade_out_ns: u64,
) -> Result<(), DispatchInStructError> {
    dispatch_in_struct(
        session,
        proto::SET_DEFAULT_TARGET,
        &SetDefaultTargetIn {
            target,
            _pad: 0,
            fade_in_ns,
            fade_out_ns,
        },
    )
}

pub fn get_default_target(session: SessionHandle) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::GET_DEFAULT_TARGET)
}

pub fn get_audio_output_mode(
    session: SessionHandle,
    target: u32,
) -> Result<u32, DispatchInU32OutU32Error> {
    dispatch_in_u32_out_u32(session, proto::GET_AUDIO_OUTPUT_MODE, target)
}

pub fn set_audio_output_mode(
    session: SessionHandle,
    target: u32,
    mode: u32,
) -> Result<(), DispatchInStructError> {
    dispatch_in_struct(
        session,
        proto::SET_AUDIO_OUTPUT_MODE,
        &TargetModeIn { target, mode },
    )
}

pub fn set_force_mute_policy(
    session: SessionHandle,
    policy: u32,
) -> Result<(), DispatchInU32Error> {
    dispatch_in_u32(session, proto::SET_FORCE_MUTE_POLICY, policy)
}

pub fn get_force_mute_policy(session: SessionHandle) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::GET_FORCE_MUTE_POLICY)
}

pub fn get_output_mode_setting(
    session: SessionHandle,
    target: u32,
) -> Result<u32, DispatchInU32OutU32Error> {
    dispatch_in_u32_out_u32(session, proto::GET_OUTPUT_MODE_SETTING, target)
}

pub fn set_output_mode_setting(
    session: SessionHandle,
    target: u32,
    mode: u32,
) -> Result<(), DispatchInStructError> {
    dispatch_in_struct(
        session,
        proto::SET_OUTPUT_MODE_SETTING,
        &TargetModeIn { target, mode },
    )
}

pub fn set_output_target(session: SessionHandle, target: u32) -> Result<(), DispatchInU32Error> {
    dispatch_in_u32(session, proto::SET_OUTPUT_TARGET, target)
}

pub fn set_input_target_force_enabled(
    session: SessionHandle,
    enable: bool,
) -> Result<(), DispatchInBoolError> {
    dispatch_in_bool(session, proto::SET_INPUT_TARGET_FORCE_ENABLED, enable)
}

pub fn set_headphone_output_level_mode(
    session: SessionHandle,
    mode: u32,
) -> Result<(), DispatchInU32Error> {
    dispatch_in_u32(session, proto::SET_HEADPHONE_OUTPUT_LEVEL_MODE, mode)
}

pub fn get_headphone_output_level_mode(session: SessionHandle) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::GET_HEADPHONE_OUTPUT_LEVEL_MODE)
}

pub fn acquire_audio_volume_update_event_for_play_report(
    session: SessionHandle,
) -> Result<u32, DispatchEventError> {
    dispatch_event(
        session,
        proto::ACQUIRE_AUDIO_VOLUME_UPDATE_EVENT_FOR_PLAY_REPORT,
    )
}

pub fn acquire_audio_output_device_update_event_for_play_report(
    session: SessionHandle,
) -> Result<u32, DispatchEventError> {
    dispatch_event(
        session,
        proto::ACQUIRE_AUDIO_OUTPUT_DEVICE_UPDATE_EVENT_FOR_PLAY_REPORT,
    )
}

pub fn get_audio_output_target_for_play_report(
    session: SessionHandle,
) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::GET_AUDIO_OUTPUT_TARGET_FOR_PLAY_REPORT)
}

pub fn notify_headphone_volume_warning_displayed_event(
    session: SessionHandle,
) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(
        session,
        proto::NOTIFY_HEADPHONE_VOLUME_WARNING_DISPLAYED_EVENT,
    )
}

pub fn set_system_output_master_volume(
    session: SessionHandle,
    volume: f32,
) -> Result<(), DispatchInF32Error> {
    dispatch_in_f32(session, proto::SET_SYSTEM_OUTPUT_MASTER_VOLUME, volume)
}

pub fn get_system_output_master_volume(session: SessionHandle) -> Result<f32, DispatchOutF32Error> {
    dispatch_out_f32(session, proto::GET_SYSTEM_OUTPUT_MASTER_VOLUME)
}

pub fn get_active_output_target(session: SessionHandle) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::GET_ACTIVE_OUTPUT_TARGET)
}
