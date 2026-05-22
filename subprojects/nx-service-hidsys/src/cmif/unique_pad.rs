//! UniquePad event, query, LED, and USB commands.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, OutHandleAttr, Session};

use super::AcquireEventError;
use crate::{
    dispatch::{dispatch_in, dispatch_in_out, dispatch_out},
    proto,
    types::{
        BtdrvAddress, SetNotificationLedPatternIn, SetNotificationLedPatternWithTimeoutIn,
        UniquePadId, UniquePadSerialNumber,
    },
};

// ---------------------------------------------------------------------------
// UniquePad events / enumeration
// ---------------------------------------------------------------------------

/// AcquireUniquePadConnectionEventHandle (cmd 702). Returns a copy handle.
pub(crate) fn acquire_unique_pad_connection_event_handle(
    service: &Session,
) -> Result<u32, AcquireEventError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::ACQUIRE_UNIQUE_PAD_CONNECTION_EVENT_HANDLE)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AcquireEventError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(AcquireEventError::MissingHandle);
    }

    Ok(result.copy_handles[0])
}

/// GetUniquePadIds (cmd 703). Returns the number of IDs written.
pub(crate) fn get_unique_pad_ids(
    service: &Session,
    out_pads: &mut [UniquePadId],
) -> Result<i64, DispatchError> {
    // SAFETY: `out_pads` is a valid `&mut` slice; viewing it as mutable bytes
    // for the OUT pointer buffer is sound.
    let buf_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            out_pads.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(out_pads),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::GET_UNIQUE_PAD_IDS)
        .out_buffer(buf_bytes, BufferAttr::HIPC_POINTER)
        .out_size(size_of::<i64>())
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least 8 bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<i64>()) })
}

/// AcquireJoyDetachOnBluetoothOffEventHandle (cmd 751). Sends PID + ARUID,
/// returns a copy handle.
pub(crate) fn acquire_joy_detach_on_bluetooth_off_event_handle(
    service: &Session,
    aruid: u64,
) -> Result<u32, AcquireEventError> {
    // SAFETY: `aruid` is a `Copy` value on the stack, valid until `.send()`.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const aruid).cast::<u8>(), size_of::<u64>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::ACQUIRE_JOY_DETACH_ON_BLUETOOTH_OFF_EVENT_HANDLE)
        .in_raw(in_bytes)
        .send_pid()
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AcquireEventError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(AcquireEventError::MissingHandle);
    }

    Ok(result.copy_handles[0])
}

// ---------------------------------------------------------------------------
// UniquePad device queries
// ---------------------------------------------------------------------------

/// GetUniquePadBluetoothAddress (cmd 805, 3.0.0+).
pub(crate) fn get_unique_pad_bluetooth_address(
    service: &Session,
    unique_pad_id: UniquePadId,
) -> Result<BtdrvAddress, DispatchError> {
    dispatch_in_out(
        service,
        proto::GET_UNIQUE_PAD_BLUETOOTH_ADDRESS,
        &unique_pad_id,
    )
}

/// DisconnectUniquePad (cmd 806, 3.0.0+).
pub(crate) fn disconnect_unique_pad(
    service: &Session,
    unique_pad_id: UniquePadId,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::DISCONNECT_UNIQUE_PAD, &unique_pad_id)
}

/// GetUniquePadType (cmd 807, 5.0.0+). Returns the raw u64 value.
pub(crate) fn get_unique_pad_type(
    service: &Session,
    unique_pad_id: UniquePadId,
) -> Result<u64, DispatchError> {
    dispatch_in_out(service, proto::GET_UNIQUE_PAD_TYPE, &unique_pad_id)
}

/// GetUniquePadInterface (cmd 808, 5.0.0+). Returns the raw u64 value.
pub(crate) fn get_unique_pad_interface(
    service: &Session,
    unique_pad_id: UniquePadId,
) -> Result<u64, DispatchError> {
    dispatch_in_out(service, proto::GET_UNIQUE_PAD_INTERFACE, &unique_pad_id)
}

/// GetUniquePadSerialNumber (cmd 809, 5.0.0+).
pub(crate) fn get_unique_pad_serial_number(
    service: &Session,
    unique_pad_id: UniquePadId,
) -> Result<UniquePadSerialNumber, DispatchError> {
    dispatch_in_out(service, proto::GET_UNIQUE_PAD_SERIAL_NUMBER, &unique_pad_id)
}

/// GetUniquePadControllerNumber (cmd 810, 5.0.0+).
pub(crate) fn get_unique_pad_controller_number(
    service: &Session,
    unique_pad_id: UniquePadId,
) -> Result<u64, DispatchError> {
    dispatch_in_out(
        service,
        proto::GET_UNIQUE_PAD_CONTROLLER_NUMBER,
        &unique_pad_id,
    )
}

// ---------------------------------------------------------------------------
// Notification LED
// ---------------------------------------------------------------------------

/// SetNotificationLedPattern (cmd 830, 7.0.0+).
pub(crate) fn set_notification_led_pattern(
    service: &Session,
    pattern: &crate::types::NotificationLedPattern,
    unique_pad_id: UniquePadId,
) -> Result<(), DispatchError> {
    let input = SetNotificationLedPatternIn {
        pattern: *pattern,
        unique_pad_id,
    };
    dispatch_in(service, proto::SET_NOTIFICATION_LED_PATTERN, &input)
}

/// SetNotificationLedPatternWithTimeout (cmd 831, 9.0.0+).
pub(crate) fn set_notification_led_pattern_with_timeout(
    service: &Session,
    pattern: &crate::types::NotificationLedPattern,
    unique_pad_id: UniquePadId,
    timeout: u64,
) -> Result<(), DispatchError> {
    let input = SetNotificationLedPatternWithTimeoutIn {
        pattern: *pattern,
        unique_pad_id,
        timeout,
    };
    dispatch_in(
        service,
        proto::SET_NOTIFICATION_LED_PATTERN_WITH_TIMEOUT,
        &input,
    )
}

// ---------------------------------------------------------------------------
// USB
// ---------------------------------------------------------------------------

/// IsUsbFullKeyControllerEnabled (cmd 850, 3.0.0+).
pub(crate) fn is_usb_full_key_controller_enabled(service: &Session) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_out(service, proto::IS_USB_FULL_KEY_CONTROLLER_ENABLED)?;
    Ok(out & 1 != 0)
}

/// EnableUsbFullKeyController (cmd 851, 3.0.0+).
pub(crate) fn enable_usb_full_key_controller(
    service: &Session,
    flag: bool,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::ENABLE_USB_FULL_KEY_CONTROLLER,
        &(flag as u8),
    )
}

/// IsUsbConnected (cmd 852, 3.0.0+).
pub(crate) fn is_usb_connected(
    service: &Session,
    unique_pad_id: UniquePadId,
) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_in_out(service, proto::IS_USB_CONNECTED, &unique_pad_id.id)?;
    Ok(out & 1 != 0)
}
