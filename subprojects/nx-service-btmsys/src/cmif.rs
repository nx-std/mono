//! CMIF protocol operations for the Bluetooth Manager System service.

use core::{
    mem::size_of,
    ptr,
};

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    OutHandleAttr,
    Session,
};

use crate::{
    dispatch::{
        dispatch_in,
        dispatch_no_io,
        dispatch_out,
    },
    proto,
    types::{
        BtdrvAddress,
        BtmAudioDevice,
    },
};

// ---------------------------------------------------------------------------
// Root service commands
// ---------------------------------------------------------------------------

/// Gets the IBtmSystemCore sub-object (cmd 0).
pub(crate) fn get_core(service: &Session) -> Result<u32, GetCoreError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::GET_CORE)
        .send(&mut ipc_buf)
        .map_err(GetCoreError::Dispatch)?;

    let Some(handle) = result.move_handles.first().copied() else {
        return Err(GetCoreError::MissingHandle);
    };

    Ok(handle)
}

// ---------------------------------------------------------------------------
// IBtmSystemCore — Gamepad pairing commands
// ---------------------------------------------------------------------------

/// StartGamepadPairing (cmd 0).
pub(crate) fn start_gamepad_pairing(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::START_GAMEPAD_PAIRING)
}

/// CancelGamepadPairing (cmd 1).
pub(crate) fn cancel_gamepad_pairing(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::CANCEL_GAMEPAD_PAIRING)
}

/// ClearGamepadPairingDatabase (cmd 2).
pub(crate) fn clear_gamepad_pairing_database(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::CLEAR_GAMEPAD_PAIRING_DATABASE)
}

/// GetPairedGamepadCount (cmd 3).
pub(crate) fn get_paired_gamepad_count(service: &Session) -> Result<u8, DispatchError> {
    dispatch_out(service, proto::GET_PAIRED_GAMEPAD_COUNT)
}

// ---------------------------------------------------------------------------
// IBtmSystemCore — Radio commands
// ---------------------------------------------------------------------------

/// EnableRadio (cmd 4).
pub(crate) fn enable_radio(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::ENABLE_RADIO)
}

/// DisableRadio (cmd 5).
pub(crate) fn disable_radio(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::DISABLE_RADIO)
}

/// GetRadioOnOff (cmd 6).
pub(crate) fn get_radio_on_off(service: &Session) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(service, proto::GET_RADIO_ON_OFF)?;
    Ok(val & 1 != 0)
}

/// AcquireRadioEvent (cmd 7, 3.0.0+).
///
/// Returns a copy handle for the radio event and verifies the out flag.
pub(crate) fn acquire_radio_event(service: &Session) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_RADIO_EVENT)
}

/// AcquireGamepadPairingEvent (cmd 8, 3.0.0+).
///
/// Returns a copy handle for the gamepad pairing event and verifies the out flag.
pub(crate) fn acquire_gamepad_pairing_event(
    service: &Session,
) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_GAMEPAD_PAIRING_EVENT)
}

/// IsGamepadPairingStarted (cmd 9, 3.0.0+).
pub(crate) fn is_gamepad_pairing_started(service: &Session) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(service, proto::IS_GAMEPAD_PAIRING_STARTED)?;
    Ok(val & 1 != 0)
}

// ---------------------------------------------------------------------------
// IBtmSystemCore — Audio device commands (13.0.0+)
// ---------------------------------------------------------------------------

/// StartAudioDeviceDiscovery (cmd 10).
pub(crate) fn start_audio_device_discovery(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::START_AUDIO_DEVICE_DISCOVERY)
}

/// StopAudioDeviceDiscovery (cmd 11).
pub(crate) fn stop_audio_device_discovery(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::STOP_AUDIO_DEVICE_DISCOVERY)
}

/// IsDiscoveryingAudioDevice (cmd 12).
pub(crate) fn is_discoverying_audio_device(service: &Session) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(service, proto::IS_DISCOVERYING_AUDIO_DEVICE)?;
    Ok(val & 1 != 0)
}

/// GetDiscoveredAudioDevice (cmd 13).
///
/// Writes discovered audio devices into the caller's buffer and returns the
/// count of entries written.
pub(crate) fn get_discovered_audio_device(
    service: &Session,
    out: &mut [BtmAudioDevice],
) -> Result<i32, DispatchError> {
    get_audio_device_list(service, out, proto::GET_DISCOVERED_AUDIO_DEVICE)
}

/// AcquireAudioDeviceConnectionEvent (cmd 14).
///
/// Returns a copy handle for the audio device connection event.
pub(crate) fn acquire_audio_device_connection_event(
    service: &Session,
) -> Result<u32, AcquireEventError> {
    acquire_event(service, proto::ACQUIRE_AUDIO_DEVICE_CONNECTION_EVENT)
}

/// ConnectAudioDevice (cmd 15).
pub(crate) fn connect_audio_device(
    service: &Session,
    addr: &BtdrvAddress,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::CONNECT_AUDIO_DEVICE, *addr)
}

/// IsConnectingAudioDevice (cmd 16).
pub(crate) fn is_connecting_audio_device(service: &Session) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(service, proto::IS_CONNECTING_AUDIO_DEVICE)?;
    Ok(val & 1 != 0)
}

/// GetConnectedAudioDevices (cmd 17).
pub(crate) fn get_connected_audio_devices(
    service: &Session,
    out: &mut [BtmAudioDevice],
) -> Result<i32, DispatchError> {
    get_audio_device_list(service, out, proto::GET_CONNECTED_AUDIO_DEVICES)
}

/// DisconnectAudioDevice (cmd 18).
pub(crate) fn disconnect_audio_device(
    service: &Session,
    addr: &BtdrvAddress,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::DISCONNECT_AUDIO_DEVICE, *addr)
}

/// AcquirePairedAudioDeviceInfoChangedEvent (cmd 19).
///
/// Returns a copy handle for the paired audio device info changed event.
pub(crate) fn acquire_paired_audio_device_info_changed_event(
    service: &Session,
) -> Result<u32, AcquireEventError> {
    acquire_event(
        service,
        proto::ACQUIRE_PAIRED_AUDIO_DEVICE_INFO_CHANGED_EVENT,
    )
}

/// GetPairedAudioDevices (cmd 20).
pub(crate) fn get_paired_audio_devices(
    service: &Session,
    out: &mut [BtmAudioDevice],
) -> Result<i32, DispatchError> {
    get_audio_device_list(service, out, proto::GET_PAIRED_AUDIO_DEVICES)
}

/// RemoveAudioDevicePairing (cmd 21).
pub(crate) fn remove_audio_device_pairing(
    service: &Session,
    addr: &BtdrvAddress,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::REMOVE_AUDIO_DEVICE_PAIRING, *addr)
}

/// RequestAudioDeviceConnectionRejection (cmd 22).
///
/// Sends PID and applet resource user ID.
pub(crate) fn request_audio_device_connection_rejection(
    service: &Session,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    dispatch_aruid(
        service,
        proto::REQUEST_AUDIO_DEVICE_CONNECTION_REJECTION,
        applet_resource_user_id,
    )
}

/// CancelAudioDeviceConnectionRejection (cmd 23).
///
/// Sends PID and applet resource user ID.
pub(crate) fn cancel_audio_device_connection_rejection(
    service: &Session,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    dispatch_aruid(
        service,
        proto::CANCEL_AUDIO_DEVICE_CONNECTION_REJECTION,
        applet_resource_user_id,
    )
}

// ---------------------------------------------------------------------------
// Shared dispatch helpers
// ---------------------------------------------------------------------------

/// Dispatches a command that sends PID and an applet resource user ID.
fn dispatch_aruid(
    service: &Session,
    cmd_id: u32,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    // SAFETY: `applet_resource_user_id` is a `Copy` value on the stack, valid
    // until `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const applet_resource_user_id).cast::<u8>(),
            size_of::<u64>(),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send_pid()
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Dispatches a command that returns an array of audio devices via HipcPointer
/// output buffer and an i32 count.
fn get_audio_device_list(
    service: &Session,
    out: &mut [BtmAudioDevice],
    cmd_id: u32,
) -> Result<i32, DispatchError> {
    // SAFETY: `out` is a valid `&mut [BtmAudioDevice]`; viewing it as a byte
    // slice for the OUT buffer is sound, and the byte slice borrows `out`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(out.as_mut_ptr().cast::<u8>(), core::mem::size_of_val(out))
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<i32>() bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

/// Dispatches a command that returns a copy handle for an event.
fn acquire_event(service: &Session, cmd_id: u32) -> Result<u32, AcquireEventError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AcquireEventError::Dispatch)?;

    let Some(handle) = result.copy_handles.first().copied() else {
        return Err(AcquireEventError::MissingHandle);
    };

    Ok(handle)
}

/// Dispatches a command that returns a copy handle for an event plus an out
/// flag byte that must be nonzero (libnx ShouldNotHappen check).
fn acquire_event_with_flag(
    service: &Session,
    cmd_id: u32,
) -> Result<u32, AcquireEventWithFlagError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<u8>())
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AcquireEventWithFlagError::Dispatch)?;

    // SAFETY: response payload is at least 1 byte.
    let flag: u8 = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };

    if flag == 0 {
        return Err(AcquireEventWithFlagError::FlagNotSet);
    }

    let Some(handle) = result.copy_handles.first().copied() else {
        return Err(AcquireEventWithFlagError::MissingHandle);
    };

    Ok(handle)
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned by [`get_core`].
#[derive(Debug, thiserror::Error)]
pub enum GetCoreError {
    #[error("failed to dispatch GetCore")]
    Dispatch(#[source] DispatchError),
    #[error("GetCore response did not include expected move handle")]
    MissingHandle,
}

/// Error returned by event acquisition commands that also return a flag.
#[derive(Debug, thiserror::Error)]
pub enum AcquireEventWithFlagError {
    #[error("failed to dispatch event acquisition")]
    Dispatch(#[source] DispatchError),
    #[error("event acquisition response flag was not set")]
    FlagNotSet,
    #[error("event acquisition response did not include expected copy handle")]
    MissingHandle,
}

/// Error returned by event acquisition commands (copy handle only).
#[derive(Debug, thiserror::Error)]
pub enum AcquireEventError {
    #[error("failed to dispatch event acquisition")]
    Dispatch(#[source] DispatchError),
    #[error("event acquisition response did not include expected copy handle")]
    MissingHandle,
}
