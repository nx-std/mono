//! CMIF protocol operations for the HID Debug service.

use core::{mem::size_of, ptr};

use nx_sf::service::{BufferAttr, DispatchError, OutHandleAttr, Session};

use crate::{
    dispatch::{dispatch_in, dispatch_in_out, dispatch_no_io},
    proto,
    types::{
        AbstractedPadHandle, AbstractedPadState, ApplyHdlsNpadAssignmentIn, DebugPadAutoPilotState,
        HdlsDeviceInfo, HdlsDeviceInfoV7, HdlsHandle, HdlsSessionId, HidTouchState,
        KeyboardAutoPilotState, MouseAutoPilotState, ReadSerialFlashIn, SetAutoPilotVirtualPadIn,
        SetHdlsStateIn, SetHdlsStateV7In, SetHdlsStateV9In, SleepButtonAutoPilotState, UniquePadId,
        UpdateControllerColorIn, UpdateDesignInfoIn, WriteSerialFlashIn,
    },
};

// ---------------------------------------------------------------------------
// AutoPilot commands
// ---------------------------------------------------------------------------

/// SetDebugPadAutoPilotState (cmd 1).
pub(crate) fn set_debug_pad_auto_pilot_state(
    service: &Session,
    state: &DebugPadAutoPilotState,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_DEBUG_PAD_AUTO_PILOT_STATE, state)
}

/// UnsetDebugPadAutoPilotState (cmd 2).
pub(crate) fn unset_debug_pad_auto_pilot_state(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::UNSET_DEBUG_PAD_AUTO_PILOT_STATE)
}

/// SetTouchScreenAutoPilotState (cmd 11, HipcMapAlias in buffer).
pub(crate) fn set_touch_screen_auto_pilot_state(
    service: &Session,
    states: &[HidTouchState],
) -> Result<(), DispatchError> {
    let buf_ptr = states.as_ptr().cast::<u8>();
    let buf_len = core::mem::size_of_val(states);

    service
        .dispatch(proto::SET_TOUCH_SCREEN_AUTO_PILOT_STATE)
        .buffer(
            buf_ptr,
            buf_len,
            BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send()
        .map(|_| ())
}

/// UnsetTouchScreenAutoPilotState (cmd 12).
pub(crate) fn unset_touch_screen_auto_pilot_state(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::UNSET_TOUCH_SCREEN_AUTO_PILOT_STATE)
}

/// SetMouseAutoPilotState (cmd 21).
pub(crate) fn set_mouse_auto_pilot_state(
    service: &Session,
    state: &MouseAutoPilotState,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_MOUSE_AUTO_PILOT_STATE, state)
}

/// UnsetMouseAutoPilotState (cmd 22).
pub(crate) fn unset_mouse_auto_pilot_state(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::UNSET_MOUSE_AUTO_PILOT_STATE)
}

/// SetKeyboardAutoPilotState (cmd 31).
pub(crate) fn set_keyboard_auto_pilot_state(
    service: &Session,
    state: &KeyboardAutoPilotState,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_KEYBOARD_AUTO_PILOT_STATE, state)
}

/// UnsetKeyboardAutoPilotState (cmd 32).
pub(crate) fn unset_keyboard_auto_pilot_state(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::UNSET_KEYBOARD_AUTO_PILOT_STATE)
}

/// DeactivateHomeButton (cmd 110).
pub(crate) fn deactivate_home_button(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::DEACTIVATE_HOME_BUTTON)
}

/// SetSleepButtonAutoPilotState (cmd 121).
pub(crate) fn set_sleep_button_auto_pilot_state(
    service: &Session,
    state: &SleepButtonAutoPilotState,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_SLEEP_BUTTON_AUTO_PILOT_STATE, state)
}

/// UnsetSleepButtonAutoPilotState (cmd 122).
pub(crate) fn unset_sleep_button_auto_pilot_state(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::UNSET_SLEEP_BUTTON_AUTO_PILOT_STATE)
}

// ---------------------------------------------------------------------------
// Controller color / serial flash commands
// ---------------------------------------------------------------------------

/// UpdateControllerColor (cmd 221, 3.0.0+).
pub(crate) fn update_controller_color(
    service: &Session,
    color_body: u32,
    color_buttons: u32,
    unique_pad_id: UniquePadId,
) -> Result<(), DispatchError> {
    let input = UpdateControllerColorIn {
        color_body,
        color_buttons,
        unique_pad_id,
    };
    dispatch_in(service, proto::UPDATE_CONTROLLER_COLOR, &input)
}

/// UpdateDesignInfo (cmd 224, 5.0.0+).
pub(crate) fn update_design_info(
    service: &Session,
    color_body: u32,
    color_buttons: u32,
    color_left_grip: u32,
    color_right_grip: u32,
    inval: u8,
    unique_pad_id: UniquePadId,
) -> Result<(), DispatchError> {
    let input = UpdateDesignInfoIn {
        color_body,
        color_buttons,
        color_left_grip,
        color_right_grip,
        inval,
        pad: [0; 7],
        unique_pad_id,
    };
    dispatch_in(service, proto::UPDATE_DESIGN_INFO, &input)
}

/// AcquireOperationEventHandle (cmd 228, 6.0.0+).
pub(crate) fn acquire_operation_event_handle(
    service: &Session,
    unique_pad_id: UniquePadId,
) -> Result<u32, AcquireEventError> {
    // SAFETY: `unique_pad_id` lives on the stack until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(proto::ACQUIRE_OPERATION_EVENT_HANDLE)
            .in_raw(
                (&raw const unique_pad_id).cast::<u8>(),
                size_of::<UniquePadId>(),
            )
            .out_handle(0, OutHandleAttr::Copy)
            .send()
            .map_err(AcquireEventError::Dispatch)?
    };

    if result.copy_handles.is_empty() {
        return Err(AcquireEventError::MissingHandle);
    }

    Ok(result.copy_handles[0])
}

/// ReadSerialFlash (cmd 229, 6.0.0+). Sends the raw IPC with a copy-handle-in for
/// the transfer memory. The caller is responsible for tmem lifecycle and event wait.
pub(crate) fn read_serial_flash(
    service: &Session,
    offset: u32,
    size: u64,
    unique_pad_id: UniquePadId,
    tmem_handle: u32,
) -> Result<(), DispatchError> {
    let input = ReadSerialFlashIn {
        offset,
        pad: 0,
        size,
        unique_pad_id,
    };

    // SAFETY: `input` lives on the stack until `.send()` returns.
    unsafe {
        service
            .dispatch(proto::READ_SERIAL_FLASH)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<ReadSerialFlashIn>(),
            )
            .in_handle(tmem_handle)
            .send()
            .map(|_| ())
    }
}

/// WriteSerialFlash (cmd 230, 6.0.0+). Sends the raw IPC with a copy-handle-in for
/// the transfer memory. The caller is responsible for tmem lifecycle and event wait.
pub(crate) fn write_serial_flash(
    service: &Session,
    offset: u32,
    tmem_size: u64,
    size: u64,
    unique_pad_id: UniquePadId,
    tmem_handle: u32,
) -> Result<(), DispatchError> {
    let input = WriteSerialFlashIn {
        offset,
        pad: 0,
        tmem_size,
        size,
        unique_pad_id,
    };

    // SAFETY: `input` lives on the stack until `.send()` returns.
    unsafe {
        service
            .dispatch(proto::WRITE_SERIAL_FLASH)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<WriteSerialFlashIn>(),
            )
            .in_handle(tmem_handle)
            .send()
            .map(|_| ())
    }
}

/// GetOperationResult (cmd 231, 6.0.0+).
pub(crate) fn get_operation_result(
    service: &Session,
    unique_pad_id: UniquePadId,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::GET_OPERATION_RESULT, &unique_pad_id.id)
}

/// GetUniquePadDeviceTypeSetInternal (cmd 234, 6.0.0+).
pub(crate) fn get_unique_pad_device_type_set_internal(
    service: &Session,
    unique_pad_id: UniquePadId,
) -> Result<u32, DispatchError> {
    dispatch_in_out(
        service,
        proto::GET_UNIQUE_PAD_DEVICE_TYPE_SET_INTERNAL,
        &unique_pad_id,
    )
}

// ---------------------------------------------------------------------------
// AbstractedPad commands (5.0.0-8.1.0)
// ---------------------------------------------------------------------------

/// GetAbstractedPadHandles (cmd 301, HipcPointer out buffer).
pub(crate) fn get_abstracted_pad_handles(
    service: &Session,
    handles: &mut [AbstractedPadHandle],
) -> Result<i32, DispatchError> {
    let buf_ptr = handles.as_mut_ptr().cast::<u8>();
    let buf_len = core::mem::size_of_val(handles);

    let result = service
        .dispatch(proto::GET_ABSTRACTED_PAD_HANDLES)
        .buffer(
            buf_ptr,
            buf_len,
            BufferAttr::OUT.or(BufferAttr::HIPC_POINTER),
        )
        .out_size(size_of::<i32>())
        .send()?;

    // SAFETY: response payload is at least 4 bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

/// GetAbstractedPadState (cmd 302).
pub(crate) fn get_abstracted_pad_state(
    service: &Session,
    handle: &AbstractedPadHandle,
) -> Result<AbstractedPadState, DispatchError> {
    dispatch_in_out(service, proto::GET_ABSTRACTED_PAD_STATE, handle)
}

/// GetAbstractedPadsState (cmd 303, HipcPointer + HipcAutoSelect out buffers).
pub(crate) fn get_abstracted_pads_state(
    service: &Session,
    handles: &mut [AbstractedPadHandle],
    states: &mut [AbstractedPadState],
) -> Result<i32, DispatchError> {
    let handles_ptr = handles.as_mut_ptr().cast::<u8>();
    let handles_len = core::mem::size_of_val(handles);
    let states_ptr = states.as_mut_ptr().cast::<u8>();
    let states_len = core::mem::size_of_val(states);

    let result = service
        .dispatch(proto::GET_ABSTRACTED_PADS_STATE)
        .buffer(
            handles_ptr,
            handles_len,
            BufferAttr::OUT.or(BufferAttr::HIPC_POINTER),
        )
        .buffer(
            states_ptr,
            states_len,
            BufferAttr::OUT.or(BufferAttr::HIPC_AUTO_SELECT),
        )
        .out_size(size_of::<i32>())
        .send()?;

    // SAFETY: response payload is at least 4 bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

/// SetAutoPilotVirtualPadState (cmd 321).
pub(crate) fn set_auto_pilot_virtual_pad_state(
    service: &Session,
    abstracted_virtual_pad_id: i8,
    state: &AbstractedPadState,
) -> Result<(), DispatchError> {
    let input = SetAutoPilotVirtualPadIn {
        abstracted_virtual_pad_id,
        pad: [0; 7],
        state: *state,
    };
    dispatch_in(service, proto::SET_AUTO_PILOT_VIRTUAL_PAD_STATE, &input)
}

/// UnsetAutoPilotVirtualPadState (cmd 322).
pub(crate) fn unset_auto_pilot_virtual_pad_state(
    service: &Session,
    abstracted_virtual_pad_id: i8,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::UNSET_AUTO_PILOT_VIRTUAL_PAD_STATE,
        &(abstracted_virtual_pad_id as u8),
    )
}

/// UnsetAllAutoPilotVirtualPadState (cmd 323).
pub(crate) fn unset_all_auto_pilot_virtual_pad_state(
    service: &Session,
) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::UNSET_ALL_AUTO_PILOT_VIRTUAL_PAD_STATE)
}

// ---------------------------------------------------------------------------
// HDLS commands (7.0.0+)
// ---------------------------------------------------------------------------

/// AttachHdlsWorkBuffer \[7.0.0-12.1.0\] (cmd 324, copy-handle-in for tmem, no output).
pub(crate) fn attach_hdls_work_buffer_legacy(
    service: &Session,
    tmem_handle: u32,
    tmem_size: u64,
) -> Result<(), DispatchError> {
    // SAFETY: `tmem_size` lives on the stack until `.send()` returns.
    unsafe {
        service
            .dispatch(proto::ATTACH_HDLS_WORK_BUFFER)
            .in_raw((&raw const tmem_size).cast::<u8>(), size_of::<u64>())
            .in_handle(tmem_handle)
            .send()
            .map(|_| ())
    }
}

/// AttachHdlsWorkBuffer \[13.0.0+\] (cmd 324, copy-handle-in for tmem, out session ID).
pub(crate) fn attach_hdls_work_buffer(
    service: &Session,
    tmem_handle: u32,
    tmem_size: u64,
) -> Result<HdlsSessionId, DispatchError> {
    // SAFETY: `tmem_size` lives on the stack until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(proto::ATTACH_HDLS_WORK_BUFFER)
            .in_raw((&raw const tmem_size).cast::<u8>(), size_of::<u64>())
            .in_handle(tmem_handle)
            .out_size(size_of::<u64>())
            .send()?
    };

    // SAFETY: response payload is at least 8 bytes.
    let id = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };
    Ok(HdlsSessionId { id })
}

/// ReleaseHdlsWorkBuffer \[7.0.0-12.1.0\] (cmd 325, no io).
pub(crate) fn release_hdls_work_buffer_legacy(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::RELEASE_HDLS_WORK_BUFFER)
}

/// ReleaseHdlsWorkBuffer \[13.0.0+\] (cmd 325, in session ID).
pub(crate) fn release_hdls_work_buffer(
    service: &Session,
    session_id: &HdlsSessionId,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::RELEASE_HDLS_WORK_BUFFER, &session_id.id)
}

/// DumpHdlsNpadAssignmentState \[7.0.0-12.1.0\] (cmd 326, no io).
pub(crate) fn dump_hdls_npad_assignment_state_legacy(
    service: &Session,
) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::DUMP_HDLS_NPAD_ASSIGNMENT_STATE)
}

/// DumpHdlsNpadAssignmentState \[13.0.0+\] (cmd 326, in session ID).
pub(crate) fn dump_hdls_npad_assignment_state(
    service: &Session,
    session_id: &HdlsSessionId,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::DUMP_HDLS_NPAD_ASSIGNMENT_STATE,
        &session_id.id,
    )
}

/// DumpHdlsStates \[7.0.0-12.1.0\] (cmd 327, no io).
pub(crate) fn dump_hdls_states_legacy(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::DUMP_HDLS_STATES)
}

/// DumpHdlsStates \[13.0.0+\] (cmd 327, in session ID).
pub(crate) fn dump_hdls_states(
    service: &Session,
    session_id: &HdlsSessionId,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::DUMP_HDLS_STATES, &session_id.id)
}

/// ApplyHdlsNpadAssignmentState \[7.0.0-12.1.0\] (cmd 328, in flag).
pub(crate) fn apply_hdls_npad_assignment_state_legacy(
    service: &Session,
    flag: bool,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::APPLY_HDLS_NPAD_ASSIGNMENT_STATE,
        &(flag as u8),
    )
}

/// ApplyHdlsNpadAssignmentState \[13.0.0+\] (cmd 328, in flag + session ID).
pub(crate) fn apply_hdls_npad_assignment_state(
    service: &Session,
    flag: bool,
    session_id: &HdlsSessionId,
) -> Result<(), DispatchError> {
    let input = ApplyHdlsNpadAssignmentIn {
        flag: flag as u8,
        pad: [0; 7],
        session_id: *session_id,
    };
    dispatch_in(service, proto::APPLY_HDLS_NPAD_ASSIGNMENT_STATE, &input)
}

/// ApplyHdlsStateList \[7.0.0-12.1.0\] (cmd 329, no io).
pub(crate) fn apply_hdls_state_list_legacy(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::APPLY_HDLS_STATE_LIST)
}

/// ApplyHdlsStateList \[13.0.0+\] (cmd 329, in session ID).
pub(crate) fn apply_hdls_state_list(
    service: &Session,
    session_id: &HdlsSessionId,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::APPLY_HDLS_STATE_LIST, &session_id.id)
}

/// AttachHdlsVirtualDevice \[7.0.0-8.1.0\] (cmd 330, V7 device info).
pub(crate) fn attach_hdls_virtual_device_v7(
    service: &Session,
    info: &HdlsDeviceInfoV7,
) -> Result<HdlsHandle, DispatchError> {
    dispatch_in_out(service, proto::ATTACH_HDLS_VIRTUAL_DEVICE, info)
}

/// AttachHdlsVirtualDevice \[9.0.0+\] (cmd 330).
pub(crate) fn attach_hdls_virtual_device(
    service: &Session,
    info: &HdlsDeviceInfo,
) -> Result<HdlsHandle, DispatchError> {
    dispatch_in_out(service, proto::ATTACH_HDLS_VIRTUAL_DEVICE, info)
}

/// DetachHdlsVirtualDevice (cmd 331).
pub(crate) fn detach_hdls_virtual_device(
    service: &Session,
    handle: &HdlsHandle,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::DETACH_HDLS_VIRTUAL_DEVICE, &handle.handle)
}

/// SetHdlsState \[7.0.0-8.1.0\] (cmd 332, V7 wire layout: state before handle).
pub(crate) fn set_hdls_state_v7(
    service: &Session,
    handle: &HdlsHandle,
    state: &crate::types::HdlsStateV7,
) -> Result<(), DispatchError> {
    let input = SetHdlsStateV7In {
        state: *state,
        handle: *handle,
    };
    dispatch_in(service, proto::SET_HDLS_STATE, &input)
}

/// SetHdlsState \[9.0.0-11.0.1\] (cmd 332, V9 wire layout: handle before state).
pub(crate) fn set_hdls_state_v9(
    service: &Session,
    handle: &HdlsHandle,
    state: &crate::types::HdlsStateV9,
) -> Result<(), DispatchError> {
    let input = SetHdlsStateV9In {
        handle: *handle,
        state: *state,
    };
    dispatch_in(service, proto::SET_HDLS_STATE, &input)
}

/// SetHdlsState \[12.0.0+\] (cmd 332).
pub(crate) fn set_hdls_state(
    service: &Session,
    handle: &HdlsHandle,
    state: &crate::types::HdlsState,
) -> Result<(), DispatchError> {
    let input = SetHdlsStateIn {
        handle: *handle,
        state: *state,
    };
    dispatch_in(service, proto::SET_HDLS_STATE, &input)
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned by event acquisition commands.
#[derive(Debug, thiserror::Error)]
pub enum AcquireEventError {
    #[error("failed to dispatch event acquisition")]
    Dispatch(#[source] DispatchError),
    #[error("event acquisition response did not include expected copy handle")]
    MissingHandle,
}
