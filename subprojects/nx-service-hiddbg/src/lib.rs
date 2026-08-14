//! HID Debug service (`hid:dbg`) implementation.
//!
//! Provides debug/autopilot functionality for input devices on the Nintendo Switch:
//! debug pad, touch screen, mouse, keyboard, sleep button override states, controller
//! color updates, serial flash access, abstracted virtual pads (5.0.0-8.1.0), and
//! HDLS virtual controllers (7.0.0+).
//!
//! Many commands have hosversion-dependent wire formats or availability windows.
//! Per IC-4 (hosversion-unaware), paired method variants are exposed and the caller
//! selects based on the system version:
//!
//! - HDLS work buffer: `*_legacy` (pre-13.0.0, no session ID) / standard (13.0.0+)
//! - HDLS device attach: `attach_hdls_virtual_device_v7` (pre-9.0.0) / standard (9.0.0+)
//! - HDLS state set: `set_hdls_state_v7` / `set_hdls_state_v9` / `set_hdls_state`
//! - HDLS npad assignment apply: `*_legacy` (pre-13.0.0) / standard (13.0.0+)
//! - HDLS state list apply: `*_legacy` (pre-13.0.0) / standard (13.0.0+)
//!
//! ## Usage
//!
//! 1. Connect to the service via [`connect_cmif`].
//! 2. Call methods on [`HiddbgService`].
//! 3. The session is closed automatically on `Drop`.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{
    BorrowedSessionHandle,
    Session,
};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::AcquireEventError,
    proto::SERVICE_NAME,
    types::{
        AbstractedPadHandle,
        AbstractedPadState,
        DebugPadAutoPilotState,
        HdlsDeviceInfo,
        HdlsDeviceInfoV7,
        HdlsHandle,
        HdlsNpadAssignment,
        HdlsNpadAssignmentEntry,
        HdlsSessionId,
        HdlsState,
        HdlsStateList,
        HdlsStateListEntry,
        HdlsStateListEntryV7,
        HdlsStateListEntryV9,
        HdlsStateListV7,
        HdlsStateListV9,
        HdlsStateV7,
        HdlsStateV9,
        HidAnalogStickState,
        HidTouchState,
        HidVector,
        KeyboardAutoPilotState,
        MouseAutoPilotState,
        SleepButtonAutoPilotState,
        UniquePadId,
    },
};

/// HID Debug service wrapper.
#[repr(transparent)]
pub struct HiddbgService(Session);

impl HiddbgService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

// ---------------------------------------------------------------------------
// AutoPilot commands
// ---------------------------------------------------------------------------

impl HiddbgService {
    /// SetDebugPadAutoPilotState (cmd 1).
    #[inline]
    pub fn set_debug_pad_auto_pilot_state(
        &self,
        state: &DebugPadAutoPilotState,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_debug_pad_auto_pilot_state(&self.0, state)
    }

    /// UnsetDebugPadAutoPilotState (cmd 2).
    #[inline]
    pub fn unset_debug_pad_auto_pilot_state(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::unset_debug_pad_auto_pilot_state(&self.0)
    }

    /// SetTouchScreenAutoPilotState (cmd 11). Max 16 touch states.
    #[inline]
    pub fn set_touch_screen_auto_pilot_state(
        &self,
        states: &[HidTouchState],
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_touch_screen_auto_pilot_state(&self.0, states)
    }

    /// UnsetTouchScreenAutoPilotState (cmd 12).
    #[inline]
    pub fn unset_touch_screen_auto_pilot_state(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::unset_touch_screen_auto_pilot_state(&self.0)
    }

    /// SetMouseAutoPilotState (cmd 21).
    #[inline]
    pub fn set_mouse_auto_pilot_state(
        &self,
        state: &MouseAutoPilotState,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_mouse_auto_pilot_state(&self.0, state)
    }

    /// UnsetMouseAutoPilotState (cmd 22).
    #[inline]
    pub fn unset_mouse_auto_pilot_state(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::unset_mouse_auto_pilot_state(&self.0)
    }

    /// SetKeyboardAutoPilotState (cmd 31).
    #[inline]
    pub fn set_keyboard_auto_pilot_state(
        &self,
        state: &KeyboardAutoPilotState,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_keyboard_auto_pilot_state(&self.0, state)
    }

    /// UnsetKeyboardAutoPilotState (cmd 32).
    #[inline]
    pub fn unset_keyboard_auto_pilot_state(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::unset_keyboard_auto_pilot_state(&self.0)
    }

    /// DeactivateHomeButton (cmd 110).
    #[inline]
    pub fn deactivate_home_button(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::deactivate_home_button(&self.0)
    }

    /// SetSleepButtonAutoPilotState (cmd 121).
    #[inline]
    pub fn set_sleep_button_auto_pilot_state(
        &self,
        state: &SleepButtonAutoPilotState,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_sleep_button_auto_pilot_state(&self.0, state)
    }

    /// UnsetSleepButtonAutoPilotState (cmd 122).
    #[inline]
    pub fn unset_sleep_button_auto_pilot_state(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::unset_sleep_button_auto_pilot_state(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Controller color / serial flash commands
// ---------------------------------------------------------------------------

impl HiddbgService {
    /// UpdateControllerColor (cmd 221, 3.0.0+).
    #[inline]
    pub fn update_controller_color(
        &self,
        color_body: u32,
        color_buttons: u32,
        unique_pad_id: UniquePadId,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::update_controller_color(&self.0, color_body, color_buttons, unique_pad_id)
    }

    /// UpdateDesignInfo (cmd 224, 5.0.0+).
    #[inline]
    pub fn update_design_info(
        &self,
        color_body: u32,
        color_buttons: u32,
        color_left_grip: u32,
        color_right_grip: u32,
        inval: u8,
        unique_pad_id: UniquePadId,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::update_design_info(
            &self.0,
            color_body,
            color_buttons,
            color_left_grip,
            color_right_grip,
            inval,
            unique_pad_id,
        )
    }

    /// AcquireOperationEventHandle (cmd 228, 6.0.0+). Returns a copy handle for
    /// the operation event. The caller wraps it in an event object.
    #[inline]
    pub fn acquire_operation_event_handle(
        &self,
        unique_pad_id: UniquePadId,
    ) -> Result<u32, AcquireEventError> {
        cmif::acquire_operation_event_handle(&self.0, unique_pad_id)
    }

    /// ReadSerialFlash (cmd 229, 6.0.0+). Raw IPC — the caller provides the transfer
    /// memory handle and is responsible for event wait and tmem lifecycle.
    #[inline]
    pub fn read_serial_flash(
        &self,
        offset: u32,
        size: u64,
        unique_pad_id: UniquePadId,
        tmem_handle: u32,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::read_serial_flash(&self.0, offset, size, unique_pad_id, tmem_handle)
    }

    /// WriteSerialFlash (cmd 230, 6.0.0+). Raw IPC — the caller provides the transfer
    /// memory handle and is responsible for event wait and tmem lifecycle.
    #[inline]
    pub fn write_serial_flash(
        &self,
        offset: u32,
        tmem_size: u64,
        size: u64,
        unique_pad_id: UniquePadId,
        tmem_handle: u32,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::write_serial_flash(&self.0, offset, tmem_size, size, unique_pad_id, tmem_handle)
    }

    /// GetOperationResult (cmd 231, 6.0.0+).
    #[inline]
    pub fn get_operation_result(
        &self,
        unique_pad_id: UniquePadId,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_operation_result(&self.0, unique_pad_id)
    }

    /// GetUniquePadDeviceTypeSetInternal (cmd 234, 6.0.0+).
    /// Returns the raw u32 output. On 9.0.0+ only the low byte is meaningful.
    #[inline]
    pub fn get_unique_pad_device_type_set_internal(
        &self,
        unique_pad_id: UniquePadId,
    ) -> Result<u32, nx_sf::service::DispatchError> {
        cmif::get_unique_pad_device_type_set_internal(&self.0, unique_pad_id)
    }
}

// ---------------------------------------------------------------------------
// AbstractedPad commands (5.0.0-8.1.0)
// ---------------------------------------------------------------------------

impl HiddbgService {
    /// GetAbstractedPadHandles (cmd 301, 5.0.0-8.1.0).
    /// Returns the number of handles written to `handles`.
    #[inline]
    pub fn get_abstracted_pad_handles(
        &self,
        handles: &mut [AbstractedPadHandle],
    ) -> Result<i32, nx_sf::service::DispatchError> {
        cmif::get_abstracted_pad_handles(&self.0, handles)
    }

    /// GetAbstractedPadState (cmd 302, 5.0.0-8.1.0).
    #[inline]
    pub fn get_abstracted_pad_state(
        &self,
        handle: &AbstractedPadHandle,
    ) -> Result<AbstractedPadState, nx_sf::service::DispatchError> {
        cmif::get_abstracted_pad_state(&self.0, handle)
    }

    /// GetAbstractedPadsState (cmd 303, 5.0.0-8.1.0).
    /// Returns the number of entries written to both `handles` and `states`.
    #[inline]
    pub fn get_abstracted_pads_state(
        &self,
        handles: &mut [AbstractedPadHandle],
        states: &mut [AbstractedPadState],
    ) -> Result<i32, nx_sf::service::DispatchError> {
        cmif::get_abstracted_pads_state(&self.0, handles, states)
    }

    /// SetAutoPilotVirtualPadState (cmd 321, 5.0.0-8.1.0).
    #[inline]
    pub fn set_auto_pilot_virtual_pad_state(
        &self,
        abstracted_virtual_pad_id: i8,
        state: &AbstractedPadState,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_auto_pilot_virtual_pad_state(&self.0, abstracted_virtual_pad_id, state)
    }

    /// UnsetAutoPilotVirtualPadState (cmd 322, 5.0.0-8.1.0).
    #[inline]
    pub fn unset_auto_pilot_virtual_pad_state(
        &self,
        abstracted_virtual_pad_id: i8,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::unset_auto_pilot_virtual_pad_state(&self.0, abstracted_virtual_pad_id)
    }

    /// UnsetAllAutoPilotVirtualPadState (cmd 323).
    #[inline]
    pub fn unset_all_auto_pilot_virtual_pad_state(
        &self,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::unset_all_auto_pilot_virtual_pad_state(&self.0)
    }
}

// ---------------------------------------------------------------------------
// HDLS commands (7.0.0+)
// ---------------------------------------------------------------------------

impl HiddbgService {
    /// AttachHdlsWorkBuffer \[7.0.0-12.1.0\] (cmd 324).
    /// The caller provides the transfer memory handle and its size.
    #[inline]
    pub fn attach_hdls_work_buffer_legacy(
        &self,
        tmem_handle: u32,
        tmem_size: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::attach_hdls_work_buffer_legacy(&self.0, tmem_handle, tmem_size)
    }

    /// AttachHdlsWorkBuffer \[13.0.0+\] (cmd 324). Returns the session ID.
    #[inline]
    pub fn attach_hdls_work_buffer(
        &self,
        tmem_handle: u32,
        tmem_size: u64,
    ) -> Result<HdlsSessionId, nx_sf::service::DispatchError> {
        cmif::attach_hdls_work_buffer(&self.0, tmem_handle, tmem_size)
    }

    /// ReleaseHdlsWorkBuffer \[7.0.0-12.1.0\] (cmd 325).
    #[inline]
    pub fn release_hdls_work_buffer_legacy(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::release_hdls_work_buffer_legacy(&self.0)
    }

    /// ReleaseHdlsWorkBuffer \[13.0.0+\] (cmd 325).
    #[inline]
    pub fn release_hdls_work_buffer(
        &self,
        session_id: &HdlsSessionId,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::release_hdls_work_buffer(&self.0, session_id)
    }

    /// DumpHdlsNpadAssignmentState \[7.0.0-12.1.0\] (cmd 326).
    /// After this call succeeds, the caller reads [`HdlsNpadAssignment`] from the
    /// transfer memory attached via [`attach_hdls_work_buffer_legacy`].
    #[inline]
    pub fn dump_hdls_npad_assignment_state_legacy(
        &self,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::dump_hdls_npad_assignment_state_legacy(&self.0)
    }

    /// DumpHdlsNpadAssignmentState \[13.0.0+\] (cmd 326).
    #[inline]
    pub fn dump_hdls_npad_assignment_state(
        &self,
        session_id: &HdlsSessionId,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::dump_hdls_npad_assignment_state(&self.0, session_id)
    }

    /// DumpHdlsStates \[7.0.0-12.1.0\] (cmd 327).
    /// After this call succeeds, the caller reads the versioned state list from
    /// transfer memory ([`HdlsStateListV7`], [`HdlsStateListV9`], or [`HdlsStateList`]).
    #[inline]
    pub fn dump_hdls_states_legacy(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::dump_hdls_states_legacy(&self.0)
    }

    /// DumpHdlsStates \[13.0.0+\] (cmd 327).
    #[inline]
    pub fn dump_hdls_states(
        &self,
        session_id: &HdlsSessionId,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::dump_hdls_states(&self.0, session_id)
    }

    /// ApplyHdlsNpadAssignmentState \[7.0.0-12.1.0\] (cmd 328).
    /// The caller writes [`HdlsNpadAssignment`] to transfer memory before calling.
    #[inline]
    pub fn apply_hdls_npad_assignment_state_legacy(
        &self,
        flag: bool,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::apply_hdls_npad_assignment_state_legacy(&self.0, flag)
    }

    /// ApplyHdlsNpadAssignmentState \[13.0.0+\] (cmd 328).
    #[inline]
    pub fn apply_hdls_npad_assignment_state(
        &self,
        flag: bool,
        session_id: &HdlsSessionId,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::apply_hdls_npad_assignment_state(&self.0, flag, session_id)
    }

    /// ApplyHdlsStateList \[7.0.0-12.1.0\] (cmd 329).
    /// The caller writes the versioned state list to transfer memory before calling.
    #[inline]
    pub fn apply_hdls_state_list_legacy(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::apply_hdls_state_list_legacy(&self.0)
    }

    /// ApplyHdlsStateList \[13.0.0+\] (cmd 329).
    #[inline]
    pub fn apply_hdls_state_list(
        &self,
        session_id: &HdlsSessionId,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::apply_hdls_state_list(&self.0, session_id)
    }

    /// AttachHdlsVirtualDevice \[7.0.0-8.1.0\] (cmd 330, V7 device info).
    #[inline]
    pub fn attach_hdls_virtual_device_v7(
        &self,
        info: &HdlsDeviceInfoV7,
    ) -> Result<HdlsHandle, nx_sf::service::DispatchError> {
        cmif::attach_hdls_virtual_device_v7(&self.0, info)
    }

    /// AttachHdlsVirtualDevice \[9.0.0+\] (cmd 330).
    #[inline]
    pub fn attach_hdls_virtual_device(
        &self,
        info: &HdlsDeviceInfo,
    ) -> Result<HdlsHandle, nx_sf::service::DispatchError> {
        cmif::attach_hdls_virtual_device(&self.0, info)
    }

    /// DetachHdlsVirtualDevice (cmd 331, 7.0.0+).
    #[inline]
    pub fn detach_hdls_virtual_device(
        &self,
        handle: &HdlsHandle,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::detach_hdls_virtual_device(&self.0, handle)
    }

    /// SetHdlsState \[7.0.0-8.1.0\] (cmd 332, V7 wire layout).
    #[inline]
    pub fn set_hdls_state_v7(
        &self,
        handle: &HdlsHandle,
        state: &HdlsStateV7,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_hdls_state_v7(&self.0, handle, state)
    }

    /// SetHdlsState \[9.0.0-11.0.1\] (cmd 332, V9 wire layout).
    #[inline]
    pub fn set_hdls_state_v9(
        &self,
        handle: &HdlsHandle,
        state: &HdlsStateV9,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_hdls_state_v9(&self.0, handle, state)
    }

    /// SetHdlsState \[12.0.0+\] (cmd 332).
    #[inline]
    pub fn set_hdls_state(
        &self,
        handle: &HdlsHandle,
        state: &HdlsState,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_hdls_state(&self.0, handle, state)
    }
}

// ---------------------------------------------------------------------------
// Connection
// ---------------------------------------------------------------------------

/// Connect to the `hid:dbg` service via CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<HiddbgService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let session = Session::new(handle, 0);

    Ok(HiddbgService(session))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get service handle")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
