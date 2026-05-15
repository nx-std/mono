//! HID System service (`hid:sys`) implementation.
//!
//! Provides system-level HID management on the Nintendo Switch: button event
//! acquisition, npad system policy, unique pad enumeration and queries,
//! notification LED control, touch screen configuration, and comprehensive
//! button remapping (both legacy opaque blobs and typed `Hidcfg*` structs).
//!
//! Many commands have hosversion-dependent availability or wire formats.
//! Per IC-4 (hosversion-unaware), paired method variants are exposed and the
//! caller selects based on the system version:
//!
//! - Button config: `legacy_*` (10.0.0-10.2.0, `UniquePadId` + opaque blobs)
//!   vs standard (11.0.0+, `BtdrvAddress` + typed `Hidcfg*` structs)
//! - Storage get/set: `*_deprecated` (10.0.0-12.1.0, no name)
//!   vs standard (11.0.0+, with `HidcfgStorageName`)
//!
//! ## Usage
//!
//! 1. Connect to the service via [`connect_cmif`].
//! 2. Call methods on [`HidsysService`], passing `applet_resource_user_id`
//!    where required.
//! 3. The session is closed automatically on `Drop`.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::Session;
use nx_svc::ipc::Handle;

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::AcquireEventError,
    proto::SERVICE_NAME,
    types::{
        BtdrvAddress, HidTouchScreenConfigurationForNx, HidcfgAnalogStickAssignment,
        HidcfgAnalogStickRotation, HidcfgButtonConfigEmbedded, HidcfgButtonConfigFull,
        HidcfgButtonConfigLeft, HidcfgButtonConfigRight, HidcfgDigitalButtonAssignment,
        HidcfgStorageName, HidsysButtonConfigEmbedded, HidsysButtonConfigFull,
        HidsysButtonConfigLeft, HidsysButtonConfigRight, NotificationLedPattern,
        NotificationLedPatternCycle, UniquePadId, UniquePadSerialNumber, UniquePadType,
    },
};

/// HID System service wrapper.
#[repr(transparent)]
pub struct HidsysService(Session);

impl HidsysService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> Handle {
        self.0.handle()
    }
}

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

impl HidsysService {
    /// SendKeyboardLockKeyEvent (cmd 31).
    #[inline]
    pub fn send_keyboard_lock_key_event(
        &self,
        events: u32,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::send_keyboard_lock_key_event(&self.0, events)
    }
}

// ---------------------------------------------------------------------------
// Button event handles / activation
// ---------------------------------------------------------------------------

impl HidsysService {
    /// AcquireHomeButtonEventHandle (cmd 101). Returns a copy handle for the event.
    #[inline]
    pub fn acquire_home_button_event_handle(&self, aruid: u64) -> Result<u32, AcquireEventError> {
        cmif::acquire_home_button_event_handle(&self.0, aruid)
    }

    /// ActivateHomeButton (cmd 111).
    #[inline]
    pub fn activate_home_button(&self, aruid: u64) -> Result<(), nx_sf::service::DispatchError> {
        cmif::activate_home_button(&self.0, aruid)
    }

    /// AcquireSleepButtonEventHandle (cmd 121). Returns a copy handle.
    #[inline]
    pub fn acquire_sleep_button_event_handle(&self, aruid: u64) -> Result<u32, AcquireEventError> {
        cmif::acquire_sleep_button_event_handle(&self.0, aruid)
    }

    /// ActivateSleepButton (cmd 131).
    #[inline]
    pub fn activate_sleep_button(&self, aruid: u64) -> Result<(), nx_sf::service::DispatchError> {
        cmif::activate_sleep_button(&self.0, aruid)
    }

    /// AcquireCaptureButtonEventHandle (cmd 141). Returns a copy handle.
    #[inline]
    pub fn acquire_capture_button_event_handle(
        &self,
        aruid: u64,
    ) -> Result<u32, AcquireEventError> {
        cmif::acquire_capture_button_event_handle(&self.0, aruid)
    }

    /// ActivateCaptureButton (cmd 151).
    #[inline]
    pub fn activate_capture_button(&self, aruid: u64) -> Result<(), nx_sf::service::DispatchError> {
        cmif::activate_capture_button(&self.0, aruid)
    }
}

// ---------------------------------------------------------------------------
// Npad system policy
// ---------------------------------------------------------------------------

impl HidsysService {
    /// ApplyNpadSystemCommonPolicy (cmd 303).
    #[inline]
    pub fn apply_npad_system_common_policy(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::apply_npad_system_common_policy(&self.0)
    }

    /// GetLastActiveNpad (cmd 306).
    #[inline]
    pub fn get_last_active_npad(&self) -> Result<u32, nx_sf::service::DispatchError> {
        cmif::get_last_active_npad(&self.0)
    }

    /// GetMaskedSupportedNpadStyleSet (cmd 310, 6.0.0+).
    #[inline]
    pub fn get_masked_supported_npad_style_set(
        &self,
        aruid: u64,
    ) -> Result<u32, nx_sf::service::DispatchError> {
        cmif::get_masked_supported_npad_style_set(&self.0, aruid)
    }

    /// GetNpadInterfaceType (cmd 316, 10.0.0+).
    #[inline]
    pub fn get_npad_interface_type(
        &self,
        npad_id: u32,
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_npad_interface_type(&self.0, npad_id)
    }

    /// GetNpadLeftRightInterfaceType (cmd 317, 10.0.0+).
    #[inline]
    pub fn get_npad_left_right_interface_type(
        &self,
        npad_id: u32,
    ) -> Result<(u8, u8), nx_sf::service::DispatchError> {
        cmif::get_npad_left_right_interface_type(&self.0, npad_id)
    }

    /// HasBattery (cmd 318, 10.0.0+).
    #[inline]
    pub fn has_battery(&self, npad_id: u32) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::has_battery(&self.0, npad_id)
    }

    /// HasLeftRightBattery (cmd 319, 10.0.0+).
    #[inline]
    pub fn has_left_right_battery(
        &self,
        npad_id: u32,
    ) -> Result<(bool, bool), nx_sf::service::DispatchError> {
        cmif::has_left_right_battery(&self.0, npad_id)
    }

    /// GetUniquePadsFromNpad (cmd 321, 3.0.0+). Returns the number of IDs written.
    #[inline]
    pub fn get_unique_pads_from_npad(
        &self,
        npad_id: u32,
        out_pads: &mut [UniquePadId],
    ) -> Result<i64, nx_sf::service::DispatchError> {
        cmif::get_unique_pads_from_npad(&self.0, npad_id, out_pads)
    }
}

// ---------------------------------------------------------------------------
// Applet resource / handheld control
// ---------------------------------------------------------------------------

impl HidsysService {
    /// SetAppletResourceUserId (cmd 500).
    #[inline]
    pub fn set_applet_resource_user_id(
        &self,
        aruid: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_applet_resource_user_id(&self.0, aruid)
    }

    /// EnableAppletToGetInput (cmd 503).
    #[inline]
    pub fn enable_applet_to_get_input(
        &self,
        permit_input: bool,
        aruid: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::enable_applet_to_get_input(&self.0, permit_input, aruid)
    }

    /// EnableHandheldHids (cmd 520).
    #[inline]
    pub fn enable_handheld_hids(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::enable_handheld_hids(&self.0)
    }

    /// DisableHandheldHids (cmd 521).
    #[inline]
    pub fn disable_handheld_hids(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::disable_handheld_hids(&self.0)
    }

    /// SetJoyConRailEnabled (cmd 522, 9.0.0+).
    #[inline]
    pub fn set_joy_con_rail_enabled(
        &self,
        flag: bool,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_joy_con_rail_enabled(&self.0, flag)
    }

    /// IsJoyConRailEnabled (cmd 523, 9.0.0+).
    #[inline]
    pub fn is_joy_con_rail_enabled(&self) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_joy_con_rail_enabled(&self.0)
    }

    /// IsHandheldHidsEnabled (cmd 524, 10.0.0+).
    #[inline]
    pub fn is_handheld_hids_enabled(&self) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_handheld_hids_enabled(&self.0)
    }

    /// IsJoyConAttachedOnAllRail (cmd 525, 11.0.0+).
    #[inline]
    pub fn is_joy_con_attached_on_all_rail(&self) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_joy_con_attached_on_all_rail(&self.0)
    }

    /// IsInvertedControllerConnectedOnRail (cmd 526, 19.0.0+).
    #[inline]
    pub fn is_inverted_controller_connected_on_rail(
        &self,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_inverted_controller_connected_on_rail(&self.0)
    }
}

// ---------------------------------------------------------------------------
// UniquePad events / enumeration
// ---------------------------------------------------------------------------

impl HidsysService {
    /// AcquireUniquePadConnectionEventHandle (cmd 702). Returns a copy handle.
    #[inline]
    pub fn acquire_unique_pad_connection_event_handle(&self) -> Result<u32, AcquireEventError> {
        cmif::acquire_unique_pad_connection_event_handle(&self.0)
    }

    /// GetUniquePadIds (cmd 703). Returns the number of IDs written.
    #[inline]
    pub fn get_unique_pad_ids(
        &self,
        out_pads: &mut [UniquePadId],
    ) -> Result<i64, nx_sf::service::DispatchError> {
        cmif::get_unique_pad_ids(&self.0, out_pads)
    }

    /// AcquireJoyDetachOnBluetoothOffEventHandle (cmd 751). Returns a copy handle.
    #[inline]
    pub fn acquire_joy_detach_on_bluetooth_off_event_handle(
        &self,
        aruid: u64,
    ) -> Result<u32, AcquireEventError> {
        cmif::acquire_joy_detach_on_bluetooth_off_event_handle(&self.0, aruid)
    }
}

// ---------------------------------------------------------------------------
// UniquePad device queries
// ---------------------------------------------------------------------------

impl HidsysService {
    /// GetUniquePadBluetoothAddress (cmd 805, 3.0.0+).
    #[inline]
    pub fn get_unique_pad_bluetooth_address(
        &self,
        unique_pad_id: UniquePadId,
    ) -> Result<BtdrvAddress, nx_sf::service::DispatchError> {
        cmif::get_unique_pad_bluetooth_address(&self.0, unique_pad_id)
    }

    /// DisconnectUniquePad (cmd 806, 3.0.0+).
    #[inline]
    pub fn disconnect_unique_pad(
        &self,
        unique_pad_id: UniquePadId,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::disconnect_unique_pad(&self.0, unique_pad_id)
    }

    /// GetUniquePadType (cmd 807, 5.0.0+). Returns the raw u64 value.
    #[inline]
    pub fn get_unique_pad_type(
        &self,
        unique_pad_id: UniquePadId,
    ) -> Result<u64, nx_sf::service::DispatchError> {
        cmif::get_unique_pad_type(&self.0, unique_pad_id)
    }

    /// GetUniquePadInterface (cmd 808, 5.0.0+). Returns the raw u64 value.
    #[inline]
    pub fn get_unique_pad_interface(
        &self,
        unique_pad_id: UniquePadId,
    ) -> Result<u64, nx_sf::service::DispatchError> {
        cmif::get_unique_pad_interface(&self.0, unique_pad_id)
    }

    /// GetUniquePadSerialNumber (cmd 809, 5.0.0+).
    #[inline]
    pub fn get_unique_pad_serial_number(
        &self,
        unique_pad_id: UniquePadId,
    ) -> Result<UniquePadSerialNumber, nx_sf::service::DispatchError> {
        cmif::get_unique_pad_serial_number(&self.0, unique_pad_id)
    }

    /// GetUniquePadControllerNumber (cmd 810, 5.0.0+).
    #[inline]
    pub fn get_unique_pad_controller_number(
        &self,
        unique_pad_id: UniquePadId,
    ) -> Result<u64, nx_sf::service::DispatchError> {
        cmif::get_unique_pad_controller_number(&self.0, unique_pad_id)
    }
}

// ---------------------------------------------------------------------------
// Notification LED
// ---------------------------------------------------------------------------

impl HidsysService {
    /// SetNotificationLedPattern (cmd 830, 7.0.0+).
    #[inline]
    pub fn set_notification_led_pattern(
        &self,
        pattern: &NotificationLedPattern,
        unique_pad_id: UniquePadId,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_notification_led_pattern(&self.0, pattern, unique_pad_id)
    }

    /// SetNotificationLedPatternWithTimeout (cmd 831, 9.0.0+).
    #[inline]
    pub fn set_notification_led_pattern_with_timeout(
        &self,
        pattern: &NotificationLedPattern,
        unique_pad_id: UniquePadId,
        timeout: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_notification_led_pattern_with_timeout(&self.0, pattern, unique_pad_id, timeout)
    }
}

// ---------------------------------------------------------------------------
// USB
// ---------------------------------------------------------------------------

impl HidsysService {
    /// IsUsbFullKeyControllerEnabled (cmd 850, 3.0.0+).
    #[inline]
    pub fn is_usb_full_key_controller_enabled(
        &self,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_usb_full_key_controller_enabled(&self.0)
    }

    /// EnableUsbFullKeyController (cmd 851, 3.0.0+).
    #[inline]
    pub fn enable_usb_full_key_controller(
        &self,
        flag: bool,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::enable_usb_full_key_controller(&self.0, flag)
    }

    /// IsUsbConnected (cmd 852, 3.0.0+).
    #[inline]
    pub fn is_usb_connected(
        &self,
        unique_pad_id: UniquePadId,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_usb_connected(&self.0, unique_pad_id)
    }
}

// ---------------------------------------------------------------------------
// Touch screen
// ---------------------------------------------------------------------------

impl HidsysService {
    /// GetTouchScreenDefaultConfiguration (cmd 1153, 9.0.0+).
    #[inline]
    pub fn get_touch_screen_default_configuration(
        &self,
    ) -> Result<HidTouchScreenConfigurationForNx, nx_sf::service::DispatchError> {
        cmif::get_touch_screen_default_configuration(&self.0)
    }

    /// IsFirmwareUpdateNeededForNotification (cmd 1154, 9.0.0+).
    #[inline]
    pub fn is_firmware_update_needed_for_notification(
        &self,
        unique_pad_id: UniquePadId,
        aruid: u64,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_firmware_update_needed_for_notification(&self.0, unique_pad_id, aruid)
    }
}

// ---------------------------------------------------------------------------
// Button config — legacy [10.0.0-10.2.0]
// ---------------------------------------------------------------------------

impl HidsysService {
    /// LegacyIsButtonConfigSupported (cmd 1200, 10.0.0-10.2.0).
    #[inline]
    pub fn legacy_is_button_config_supported(
        &self,
        unique_pad_id: UniquePadId,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::legacy_is_button_config_supported(&self.0, unique_pad_id)
    }

    /// LegacyDeleteButtonConfig (cmd 1201, 10.0.0-10.2.0).
    #[inline]
    pub fn legacy_delete_button_config(
        &self,
        unique_pad_id: UniquePadId,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::legacy_delete_button_config(&self.0, unique_pad_id)
    }

    /// LegacySetButtonConfigEnabled (cmd 1202, 10.0.0-10.2.0).
    #[inline]
    pub fn legacy_set_button_config_enabled(
        &self,
        unique_pad_id: UniquePadId,
        flag: bool,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::legacy_set_button_config_enabled(&self.0, unique_pad_id, flag)
    }

    /// LegacyIsButtonConfigEnabled (cmd 1203, 10.0.0-10.2.0).
    #[inline]
    pub fn legacy_is_button_config_enabled(
        &self,
        unique_pad_id: UniquePadId,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::legacy_is_button_config_enabled(&self.0, unique_pad_id)
    }

    /// LegacySetButtonConfigEmbedded (cmd 1204, 10.0.0-10.2.0).
    #[inline]
    pub fn legacy_set_button_config_embedded(
        &self,
        unique_pad_id: UniquePadId,
        config: &HidsysButtonConfigEmbedded,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::legacy_set_button_config_embedded(&self.0, unique_pad_id, config)
    }

    /// LegacySetButtonConfigFull (cmd 1205, 10.0.0-10.2.0).
    #[inline]
    pub fn legacy_set_button_config_full(
        &self,
        unique_pad_id: UniquePadId,
        config: &HidsysButtonConfigFull,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::legacy_set_button_config_full(&self.0, unique_pad_id, config)
    }

    /// LegacySetButtonConfigLeft (cmd 1206, 10.0.0-10.2.0).
    #[inline]
    pub fn legacy_set_button_config_left(
        &self,
        unique_pad_id: UniquePadId,
        config: &HidsysButtonConfigLeft,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::legacy_set_button_config_left(&self.0, unique_pad_id, config)
    }

    /// LegacySetButtonConfigRight (cmd 1207, 10.0.0-10.2.0).
    #[inline]
    pub fn legacy_set_button_config_right(
        &self,
        unique_pad_id: UniquePadId,
        config: &HidsysButtonConfigRight,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::legacy_set_button_config_right(&self.0, unique_pad_id, config)
    }

    /// LegacyGetButtonConfigEmbedded (cmd 1208, 10.0.0-10.2.0).
    #[inline]
    pub fn legacy_get_button_config_embedded(
        &self,
        unique_pad_id: UniquePadId,
        config: &mut HidsysButtonConfigEmbedded,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::legacy_get_button_config_embedded(&self.0, unique_pad_id, config)
    }

    /// LegacyGetButtonConfigFull (cmd 1209, 10.0.0-10.2.0).
    #[inline]
    pub fn legacy_get_button_config_full(
        &self,
        unique_pad_id: UniquePadId,
        config: &mut HidsysButtonConfigFull,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::legacy_get_button_config_full(&self.0, unique_pad_id, config)
    }

    /// LegacyGetButtonConfigLeft (cmd 1210, 10.0.0-10.2.0).
    #[inline]
    pub fn legacy_get_button_config_left(
        &self,
        unique_pad_id: UniquePadId,
        config: &mut HidsysButtonConfigLeft,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::legacy_get_button_config_left(&self.0, unique_pad_id, config)
    }

    /// LegacyGetButtonConfigRight (cmd 1211, 10.0.0-10.2.0).
    #[inline]
    pub fn legacy_get_button_config_right(
        &self,
        unique_pad_id: UniquePadId,
        config: &mut HidsysButtonConfigRight,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::legacy_get_button_config_right(&self.0, unique_pad_id, config)
    }
}

// ---------------------------------------------------------------------------
// Button config — [11.0.0-17.0.1]
// ---------------------------------------------------------------------------

impl HidsysService {
    /// IsButtonConfigSupported (cmd 1200, 11.0.0-17.0.1).
    #[inline]
    pub fn is_button_config_supported(
        &self,
        addr: BtdrvAddress,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_button_config_supported(&self.0, addr)
    }

    /// IsButtonConfigEmbeddedSupported (cmd 1201, 11.0.0-17.0.1).
    #[inline]
    pub fn is_button_config_embedded_supported(
        &self,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_button_config_embedded_supported(&self.0)
    }

    /// DeleteButtonConfig (cmd 1202, 11.0.0-17.0.1).
    #[inline]
    pub fn delete_button_config(
        &self,
        addr: BtdrvAddress,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::delete_button_config(&self.0, addr)
    }

    /// DeleteButtonConfigEmbedded (cmd 1203, 11.0.0-17.0.1).
    #[inline]
    pub fn delete_button_config_embedded(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::delete_button_config_embedded(&self.0)
    }

    /// SetButtonConfigEnabled (cmd 1204, 11.0.0-17.0.1).
    #[inline]
    pub fn set_button_config_enabled(
        &self,
        addr: BtdrvAddress,
        flag: bool,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_button_config_enabled(&self.0, addr, flag)
    }

    /// SetButtonConfigEmbeddedEnabled (cmd 1205, 11.0.0-17.0.1).
    #[inline]
    pub fn set_button_config_embedded_enabled(
        &self,
        flag: bool,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_button_config_embedded_enabled(&self.0, flag)
    }

    /// IsButtonConfigEnabled (cmd 1206, 11.0.0-17.0.1).
    #[inline]
    pub fn is_button_config_enabled(
        &self,
        addr: BtdrvAddress,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_button_config_enabled(&self.0, addr)
    }

    /// IsButtonConfigEmbeddedEnabled (cmd 1207, 11.0.0-17.0.1).
    #[inline]
    pub fn is_button_config_embedded_enabled(&self) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_button_config_embedded_enabled(&self.0)
    }

    /// SetButtonConfigEmbedded (cmd 1208, 11.0.0-17.0.1).
    #[inline]
    pub fn set_button_config_embedded(
        &self,
        config: &HidcfgButtonConfigEmbedded,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_button_config_embedded(&self.0, config)
    }

    /// SetButtonConfigFull (cmd 1209, 11.0.0-17.0.1).
    #[inline]
    pub fn set_button_config_full(
        &self,
        addr: BtdrvAddress,
        config: &HidcfgButtonConfigFull,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_button_config_full(&self.0, addr, config)
    }

    /// SetButtonConfigLeft (cmd 1210, 11.0.0-17.0.1).
    #[inline]
    pub fn set_button_config_left(
        &self,
        addr: BtdrvAddress,
        config: &HidcfgButtonConfigLeft,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_button_config_left(&self.0, addr, config)
    }

    /// SetButtonConfigRight (cmd 1211, 11.0.0-17.0.1).
    #[inline]
    pub fn set_button_config_right(
        &self,
        addr: BtdrvAddress,
        config: &HidcfgButtonConfigRight,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_button_config_right(&self.0, addr, config)
    }

    /// GetButtonConfigEmbedded (cmd 1212, 11.0.0-17.0.1).
    #[inline]
    pub fn get_button_config_embedded(
        &self,
        config: &mut HidcfgButtonConfigEmbedded,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_button_config_embedded(&self.0, config)
    }

    /// GetButtonConfigFull (cmd 1213, 11.0.0-17.0.1).
    #[inline]
    pub fn get_button_config_full(
        &self,
        addr: BtdrvAddress,
        config: &mut HidcfgButtonConfigFull,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_button_config_full(&self.0, addr, config)
    }

    /// GetButtonConfigLeft (cmd 1214, 11.0.0-17.0.1).
    #[inline]
    pub fn get_button_config_left(
        &self,
        addr: BtdrvAddress,
        config: &mut HidcfgButtonConfigLeft,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_button_config_left(&self.0, addr, config)
    }

    /// GetButtonConfigRight (cmd 1215, 11.0.0-17.0.1).
    #[inline]
    pub fn get_button_config_right(
        &self,
        addr: BtdrvAddress,
        config: &mut HidcfgButtonConfigRight,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_button_config_right(&self.0, addr, config)
    }
}

// ---------------------------------------------------------------------------
// Custom button config [10.0.0+]
// ---------------------------------------------------------------------------

impl HidsysService {
    /// IsCustomButtonConfigSupported (cmd 1250, 10.0.0+).
    #[inline]
    pub fn is_custom_button_config_supported(
        &self,
        unique_pad_id: UniquePadId,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_custom_button_config_supported(&self.0, unique_pad_id)
    }

    /// IsDefaultButtonConfigEmbedded (cmd 1251, 10.0.0+).
    #[inline]
    pub fn is_default_button_config_embedded(
        &self,
        config: &HidcfgButtonConfigEmbedded,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_default_button_config_embedded(&self.0, config)
    }

    /// IsDefaultButtonConfigFull (cmd 1252, 10.0.0+).
    #[inline]
    pub fn is_default_button_config_full(
        &self,
        config: &HidcfgButtonConfigFull,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_default_button_config_full(&self.0, config)
    }

    /// IsDefaultButtonConfigLeft (cmd 1253, 10.0.0+).
    #[inline]
    pub fn is_default_button_config_left(
        &self,
        config: &HidcfgButtonConfigLeft,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_default_button_config_left(&self.0, config)
    }

    /// IsDefaultButtonConfigRight (cmd 1254, 10.0.0+).
    #[inline]
    pub fn is_default_button_config_right(
        &self,
        config: &HidcfgButtonConfigRight,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_default_button_config_right(&self.0, config)
    }

    /// IsButtonConfigStorageEmbeddedEmpty (cmd 1255, 10.0.0+).
    #[inline]
    pub fn is_button_config_storage_embedded_empty(
        &self,
        index: i32,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_button_config_storage_embedded_empty(&self.0, index)
    }

    /// IsButtonConfigStorageFullEmpty (cmd 1256, 10.0.0+).
    #[inline]
    pub fn is_button_config_storage_full_empty(
        &self,
        index: i32,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_button_config_storage_full_empty(&self.0, index)
    }

    /// IsButtonConfigStorageLeftEmpty (cmd 1257, 10.0.0+).
    #[inline]
    pub fn is_button_config_storage_left_empty(
        &self,
        index: i32,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_button_config_storage_left_empty(&self.0, index)
    }

    /// IsButtonConfigStorageRightEmpty (cmd 1258, 10.0.0+).
    #[inline]
    pub fn is_button_config_storage_right_empty(
        &self,
        index: i32,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_button_config_storage_right_empty(&self.0, index)
    }

    /// GetButtonConfigStorageEmbeddedDeprecated (cmd 1259, 10.0.0-12.1.0).
    #[inline]
    pub fn get_button_config_storage_embedded_deprecated(
        &self,
        index: i32,
        config: &mut HidcfgButtonConfigEmbedded,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_button_config_storage_embedded_deprecated(&self.0, index, config)
    }

    /// GetButtonConfigStorageFullDeprecated (cmd 1260, 10.0.0-12.1.0).
    #[inline]
    pub fn get_button_config_storage_full_deprecated(
        &self,
        index: i32,
        config: &mut HidcfgButtonConfigFull,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_button_config_storage_full_deprecated(&self.0, index, config)
    }

    /// GetButtonConfigStorageLeftDeprecated (cmd 1261, 10.0.0-12.1.0).
    #[inline]
    pub fn get_button_config_storage_left_deprecated(
        &self,
        index: i32,
        config: &mut HidcfgButtonConfigLeft,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_button_config_storage_left_deprecated(&self.0, index, config)
    }

    /// GetButtonConfigStorageRightDeprecated (cmd 1262, 10.0.0-12.1.0).
    #[inline]
    pub fn get_button_config_storage_right_deprecated(
        &self,
        index: i32,
        config: &mut HidcfgButtonConfigRight,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_button_config_storage_right_deprecated(&self.0, index, config)
    }

    /// SetButtonConfigStorageEmbeddedDeprecated (cmd 1263, 10.0.0-12.1.0).
    #[inline]
    pub fn set_button_config_storage_embedded_deprecated(
        &self,
        index: i32,
        config: &HidcfgButtonConfigEmbedded,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_button_config_storage_embedded_deprecated(&self.0, index, config)
    }

    /// SetButtonConfigStorageFullDeprecated (cmd 1264, 10.0.0-12.1.0).
    #[inline]
    pub fn set_button_config_storage_full_deprecated(
        &self,
        index: i32,
        config: &HidcfgButtonConfigFull,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_button_config_storage_full_deprecated(&self.0, index, config)
    }

    /// SetButtonConfigStorageLeftDeprecated (cmd 1265, 10.0.0-12.1.0).
    #[inline]
    pub fn set_button_config_storage_left_deprecated(
        &self,
        index: i32,
        config: &HidcfgButtonConfigLeft,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_button_config_storage_left_deprecated(&self.0, index, config)
    }

    /// SetButtonConfigStorageRightDeprecated (cmd 1266, 10.0.0-12.1.0).
    #[inline]
    pub fn set_button_config_storage_right_deprecated(
        &self,
        index: i32,
        config: &HidcfgButtonConfigRight,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_button_config_storage_right_deprecated(&self.0, index, config)
    }

    /// DeleteButtonConfigStorageEmbedded (cmd 1267, 10.0.0+).
    #[inline]
    pub fn delete_button_config_storage_embedded(
        &self,
        index: i32,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::delete_button_config_storage_embedded(&self.0, index)
    }

    /// DeleteButtonConfigStorageFull (cmd 1268, 10.0.0+).
    #[inline]
    pub fn delete_button_config_storage_full(
        &self,
        index: i32,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::delete_button_config_storage_full(&self.0, index)
    }

    /// DeleteButtonConfigStorageLeft (cmd 1269, 10.0.0+).
    #[inline]
    pub fn delete_button_config_storage_left(
        &self,
        index: i32,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::delete_button_config_storage_left(&self.0, index)
    }

    /// DeleteButtonConfigStorageRight (cmd 1270, 10.0.0+).
    #[inline]
    pub fn delete_button_config_storage_right(
        &self,
        index: i32,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::delete_button_config_storage_right(&self.0, index)
    }

    /// IsUsingCustomButtonConfig (cmd 1271, 10.0.0+).
    #[inline]
    pub fn is_using_custom_button_config(
        &self,
        unique_pad_id: UniquePadId,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_using_custom_button_config(&self.0, unique_pad_id)
    }

    /// IsAnyCustomButtonConfigEnabled (cmd 1272, 10.0.0+).
    #[inline]
    pub fn is_any_custom_button_config_enabled(
        &self,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_any_custom_button_config_enabled(&self.0)
    }

    /// SetAllCustomButtonConfigEnabled (cmd 1273, 10.0.0+).
    #[inline]
    pub fn set_all_custom_button_config_enabled(
        &self,
        aruid: u64,
        flag: bool,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_all_custom_button_config_enabled(&self.0, aruid, flag)
    }

    /// SetDefaultButtonConfig (cmd 1274, 10.0.0+).
    #[inline]
    pub fn set_default_button_config(
        &self,
        unique_pad_id: UniquePadId,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_default_button_config(&self.0, unique_pad_id)
    }

    /// SetAllDefaultButtonConfig (cmd 1275, 10.0.0+).
    #[inline]
    pub fn set_all_default_button_config(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_all_default_button_config(&self.0)
    }

    /// SetHidButtonConfigEmbedded (cmd 1276, 10.0.0+).
    #[inline]
    pub fn set_hid_button_config_embedded(
        &self,
        unique_pad_id: UniquePadId,
        config: &HidcfgButtonConfigEmbedded,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_hid_button_config_embedded(&self.0, unique_pad_id, config)
    }

    /// SetHidButtonConfigFull (cmd 1277, 10.0.0+).
    #[inline]
    pub fn set_hid_button_config_full(
        &self,
        unique_pad_id: UniquePadId,
        config: &HidcfgButtonConfigFull,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_hid_button_config_full(&self.0, unique_pad_id, config)
    }

    /// SetHidButtonConfigLeft (cmd 1278, 10.0.0+).
    #[inline]
    pub fn set_hid_button_config_left(
        &self,
        unique_pad_id: UniquePadId,
        config: &HidcfgButtonConfigLeft,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_hid_button_config_left(&self.0, unique_pad_id, config)
    }

    /// SetHidButtonConfigRight (cmd 1279, 10.0.0+).
    #[inline]
    pub fn set_hid_button_config_right(
        &self,
        unique_pad_id: UniquePadId,
        config: &HidcfgButtonConfigRight,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_hid_button_config_right(&self.0, unique_pad_id, config)
    }

    /// GetHidButtonConfigEmbedded (cmd 1280, 10.0.0+).
    #[inline]
    pub fn get_hid_button_config_embedded(
        &self,
        unique_pad_id: UniquePadId,
        config: &mut HidcfgButtonConfigEmbedded,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_hid_button_config_embedded(&self.0, unique_pad_id, config)
    }

    /// GetHidButtonConfigFull (cmd 1281, 10.0.0+).
    #[inline]
    pub fn get_hid_button_config_full(
        &self,
        unique_pad_id: UniquePadId,
        config: &mut HidcfgButtonConfigFull,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_hid_button_config_full(&self.0, unique_pad_id, config)
    }

    /// GetHidButtonConfigLeft (cmd 1282, 10.0.0+).
    #[inline]
    pub fn get_hid_button_config_left(
        &self,
        unique_pad_id: UniquePadId,
        config: &mut HidcfgButtonConfigLeft,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_hid_button_config_left(&self.0, unique_pad_id, config)
    }

    /// GetHidButtonConfigRight (cmd 1283, 10.0.0+).
    #[inline]
    pub fn get_hid_button_config_right(
        &self,
        unique_pad_id: UniquePadId,
        config: &mut HidcfgButtonConfigRight,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_hid_button_config_right(&self.0, unique_pad_id, config)
    }

    /// GetButtonConfigStorageEmbedded (cmd 1284, 11.0.0+).
    #[inline]
    pub fn get_button_config_storage_embedded(
        &self,
        index: i32,
        config: &mut HidcfgButtonConfigEmbedded,
        name: &mut HidcfgStorageName,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_button_config_storage_embedded(&self.0, index, config, name)
    }

    /// GetButtonConfigStorageFull (cmd 1285, 11.0.0+).
    #[inline]
    pub fn get_button_config_storage_full(
        &self,
        index: i32,
        config: &mut HidcfgButtonConfigFull,
        name: &mut HidcfgStorageName,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_button_config_storage_full(&self.0, index, config, name)
    }

    /// GetButtonConfigStorageLeft (cmd 1286, 11.0.0+).
    #[inline]
    pub fn get_button_config_storage_left(
        &self,
        index: i32,
        config: &mut HidcfgButtonConfigLeft,
        name: &mut HidcfgStorageName,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_button_config_storage_left(&self.0, index, config, name)
    }

    /// GetButtonConfigStorageRight (cmd 1287, 11.0.0+).
    #[inline]
    pub fn get_button_config_storage_right(
        &self,
        index: i32,
        config: &mut HidcfgButtonConfigRight,
        name: &mut HidcfgStorageName,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_button_config_storage_right(&self.0, index, config, name)
    }

    /// SetButtonConfigStorageEmbedded (cmd 1288, 11.0.0+).
    #[inline]
    pub fn set_button_config_storage_embedded(
        &self,
        index: i32,
        config: &HidcfgButtonConfigEmbedded,
        name: &HidcfgStorageName,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_button_config_storage_embedded(&self.0, index, config, name)
    }

    /// SetButtonConfigStorageFull (cmd 1289, 11.0.0+).
    #[inline]
    pub fn set_button_config_storage_full(
        &self,
        index: i32,
        config: &HidcfgButtonConfigFull,
        name: &HidcfgStorageName,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_button_config_storage_full(&self.0, index, config, name)
    }

    /// SetButtonConfigStorageLeft (cmd 1290, 11.0.0+).
    #[inline]
    pub fn set_button_config_storage_left(
        &self,
        index: i32,
        config: &HidcfgButtonConfigLeft,
        name: &HidcfgStorageName,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_button_config_storage_left(&self.0, index, config, name)
    }

    /// SetButtonConfigStorageRight (cmd 1291, 11.0.0+).
    #[inline]
    pub fn set_button_config_storage_right(
        &self,
        index: i32,
        config: &HidcfgButtonConfigRight,
        name: &HidcfgStorageName,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_button_config_storage_right(&self.0, index, config, name)
    }
}

// ---------------------------------------------------------------------------
// Connection
// ---------------------------------------------------------------------------

/// Connect to the `hid:sys` service via CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<HidsysService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    let session = Session::from_handle(handle, 0);

    Ok(HidsysService(session))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    #[error("failed to get service handle")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
}
