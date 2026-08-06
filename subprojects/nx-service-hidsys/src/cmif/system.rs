//! Keyboard, Npad system, applet resource, and handheld control commands.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Session,
};

use crate::{
    dispatch::{
        dispatch_in,
        dispatch_in_out,
        dispatch_no_io,
        dispatch_out,
    },
    proto,
    types::{
        EnableAppletToGetInputIn,
        LeftRightU8Out,
        UniquePadId,
    },
};

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

/// SendKeyboardLockKeyEvent (cmd 31).
pub(crate) fn send_keyboard_lock_key_event(
    service: &Session,
    events: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SEND_KEYBOARD_LOCK_KEY_EVENT, &events)
}

// ---------------------------------------------------------------------------
// Npad system policy
// ---------------------------------------------------------------------------

/// ApplyNpadSystemCommonPolicy (cmd 303).
pub(crate) fn apply_npad_system_common_policy(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::APPLY_NPAD_SYSTEM_COMMON_POLICY)
}

/// GetLastActiveNpad (cmd 306).
pub(crate) fn get_last_active_npad(service: &Session) -> Result<u32, DispatchError> {
    dispatch_out(service, proto::GET_LAST_ACTIVE_NPAD)
}

/// GetMaskedSupportedNpadStyleSet (cmd 310, 6.0.0+).
pub(crate) fn get_masked_supported_npad_style_set(
    service: &Session,
    aruid: u64,
) -> Result<u32, DispatchError> {
    dispatch_in_out(service, proto::GET_MASKED_SUPPORTED_NPAD_STYLE_SET, &aruid)
}

/// GetNpadInterfaceType (cmd 316, 10.0.0+).
pub(crate) fn get_npad_interface_type(
    service: &Session,
    npad_id: u32,
) -> Result<u8, DispatchError> {
    dispatch_in_out(service, proto::GET_NPAD_INTERFACE_TYPE, &npad_id)
}

/// GetNpadLeftRightInterfaceType (cmd 317, 10.0.0+).
pub(crate) fn get_npad_left_right_interface_type(
    service: &Session,
    npad_id: u32,
) -> Result<(u8, u8), DispatchError> {
    let out: LeftRightU8Out =
        dispatch_in_out(service, proto::GET_NPAD_LEFT_RIGHT_INTERFACE_TYPE, &npad_id)?;
    Ok((out.left, out.right))
}

/// HasBattery (cmd 318, 10.0.0+).
pub(crate) fn has_battery(service: &Session, npad_id: u32) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_in_out(service, proto::HAS_BATTERY, &npad_id)?;
    Ok(out & 1 != 0)
}

/// HasLeftRightBattery (cmd 319, 10.0.0+).
pub(crate) fn has_left_right_battery(
    service: &Session,
    npad_id: u32,
) -> Result<(bool, bool), DispatchError> {
    let out: LeftRightU8Out = dispatch_in_out(service, proto::HAS_LEFT_RIGHT_BATTERY, &npad_id)?;
    Ok((out.left & 1 != 0, out.right & 1 != 0))
}

/// GetUniquePadsFromNpad (cmd 321, 3.0.0+). Returns the number of IDs written.
pub(crate) fn get_unique_pads_from_npad(
    service: &Session,
    npad_id: u32,
    out_pads: &mut [UniquePadId],
) -> Result<i64, DispatchError> {
    // SAFETY: `npad_id` is a `Copy` value on the stack, valid until `.send()`.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const npad_id).cast::<u8>(), size_of::<u32>()) };
    // SAFETY: `out_pads` is a valid `&mut` slice; viewing it as mutable bytes
    // for the OUT pointer buffer is sound.
    let buf_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            out_pads.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(out_pads),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::GET_UNIQUE_PADS_FROM_NPAD)
        .in_raw(in_bytes)
        .out_buffer(buf_bytes, BufferAttr::HIPC_POINTER)
        .out_size(size_of::<i64>())
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least 8 bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<i64>()) })
}

// ---------------------------------------------------------------------------
// Applet resource / handheld control
// ---------------------------------------------------------------------------

/// SetAppletResourceUserId (cmd 500).
pub(crate) fn set_applet_resource_user_id(
    service: &Session,
    aruid: u64,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_APPLET_RESOURCE_USER_ID, &aruid)
}

/// EnableAppletToGetInput (cmd 503).
pub(crate) fn enable_applet_to_get_input(
    service: &Session,
    permit_input: bool,
    aruid: u64,
) -> Result<(), DispatchError> {
    let input = EnableAppletToGetInputIn {
        permit_input: permit_input as u8,
        pad: [0; 7],
        applet_resource_user_id: aruid,
    };
    dispatch_in(service, proto::ENABLE_APPLET_TO_GET_INPUT, &input)
}

/// EnableHandheldHids (cmd 520).
pub(crate) fn enable_handheld_hids(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::ENABLE_HANDHELD_HIDS)
}

/// DisableHandheldHids (cmd 521).
pub(crate) fn disable_handheld_hids(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::DISABLE_HANDHELD_HIDS)
}

/// SetJoyConRailEnabled (cmd 522, 9.0.0+).
pub(crate) fn set_joy_con_rail_enabled(service: &Session, flag: bool) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_JOY_CON_RAIL_ENABLED, &(flag as u8))
}

/// IsJoyConRailEnabled (cmd 523, 9.0.0+).
pub(crate) fn is_joy_con_rail_enabled(service: &Session) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_out(service, proto::IS_JOY_CON_RAIL_ENABLED)?;
    Ok(out & 1 != 0)
}

/// IsHandheldHidsEnabled (cmd 524, 10.0.0+).
pub(crate) fn is_handheld_hids_enabled(service: &Session) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_out(service, proto::IS_HANDHELD_HIDS_ENABLED)?;
    Ok(out & 1 != 0)
}

/// IsJoyConAttachedOnAllRail (cmd 525, 11.0.0+).
pub(crate) fn is_joy_con_attached_on_all_rail(service: &Session) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_out(service, proto::IS_JOY_CON_ATTACHED_ON_ALL_RAIL)?;
    Ok(out & 1 != 0)
}

/// IsInvertedControllerConnectedOnRail (cmd 526, 19.0.0+).
pub(crate) fn is_inverted_controller_connected_on_rail(
    service: &Session,
) -> Result<bool, DispatchError> {
    let out: u8 = dispatch_out(service, proto::IS_INVERTED_CONTROLLER_CONNECTED_ON_RAIL)?;
    Ok(out & 1 != 0)
}
