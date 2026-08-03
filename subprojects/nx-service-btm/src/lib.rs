//! Bluetooth Manager service (`btm`) implementation.
//!
//! Provides Bluetooth device management, radio control, BLE scanning,
//! GATT service discovery, and BLE connection/pairing for the Nintendo Switch.
//!
//! Many commands have hosversion-dependent wire formats or command IDs.
//! Per IC-4 (hosversion-unaware), paired `_legacy` / versioned method variants
//! are exposed and the caller selects based on the system version:
//!
//! - Pre-3.0.0 event acquisition: `*_legacy` (no out flag)
//! - 3.0.0+ event acquisition: standard (with out flag validation)
//! - Pre-9.0.0 LlrNotify: `llr_notify_legacy` (address only)
//! - 9.0.0+ LlrNotify: `llr_notify` (address + unk)
//! - Pre-13.0.0 device/host property: `*_legacy` (V1 wire layout)
//! - 13.0.0+ device/host property: standard (V13 wire layout)
//! - Device condition: four versioned getters (V100/V510/V800/V900) plus 13.0.0+
//! - 5.0.0-5.0.2 BLE commands: `*_legacy` (old command IDs)
//! - 5.1.0+ BLE commands: standard (new command IDs)
//!
//! ## Usage
//!
//! 1. Connect to the service via [`connect_cmif`].
//! 2. Call methods on [`BtmService`].
//! 3. The session is closed automatically on `Drop`.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{BorrowedSessionHandle, Session};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::{AcquireEventError, AcquireEventWithFlagError},
    proto::SERVICE_NAME,
    types::{
        BtdrvAddress, BtdrvBleAdvertisePacketParameter, BtdrvBleConnectionInfo, BtdrvBleScanResult,
        BtdrvGattAttributeUuid, BtmAudioDevice, BtmBdName, BtmBleDataPath, BtmBluetoothMode,
        BtmClassOfDevice, BtmConnectedDeviceV1, BtmConnectedDeviceV13, BtmDeviceConditionV100,
        BtmDeviceConditionV510, BtmDeviceConditionV800, BtmDeviceConditionV900, BtmDeviceInfoList,
        BtmDeviceInfoV1, BtmDeviceInfoV13, BtmDeviceProperty, BtmDevicePropertyList,
        BtmDeviceSlotMode, BtmDeviceSlotModeList, BtmGattCharacteristic,
        BtmGattClientConditionList, BtmGattDescriptor, BtmGattService, BtmHidDeviceInfo,
        BtmHostDevicePropertyV1, BtmHostDevicePropertyV13, BtmLinkKey, BtmProfile, BtmSlotMode,
        BtmState, BtmTsiMode, BtmWlanMode, BtmZeroRetransmissionList,
    },
};

/// Bluetooth Manager service wrapper.
#[repr(transparent)]
pub struct BtmService(Session);

impl BtmService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

// ---------------------------------------------------------------------------
// Core commands (always same cmd IDs)
// ---------------------------------------------------------------------------

impl BtmService {
    /// GetState (cmd 0). Returns the raw state value; cast to [`BtmState`].
    #[inline]
    pub fn get_state(&self) -> Result<u32, nx_sf::service::DispatchError> {
        cmif::get_state(&self.0)
    }

    /// GetHostDeviceProperty \[1.0.0-12.1.0\] (cmd 1).
    #[inline]
    pub fn get_host_device_property_legacy(
        &self,
    ) -> Result<BtmHostDevicePropertyV1, nx_sf::service::DispatchError> {
        cmif::get_host_device_property_legacy(&self.0)
    }

    /// GetHostDeviceProperty \[13.0.0+\] (cmd 1).
    #[inline]
    pub fn get_host_device_property(
        &self,
        out: &mut BtmHostDevicePropertyV13,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_host_device_property(&self.0, out)
    }

    /// AcquireDeviceConditionEvent (cmd 2, pre-3.0.0).
    #[inline]
    pub fn acquire_device_condition_event_legacy(&self) -> Result<u32, AcquireEventError> {
        cmif::acquire_device_condition_event_legacy(&self.0)
    }

    /// AcquireDeviceConditionEvent (cmd 2, 3.0.0+).
    #[inline]
    pub fn acquire_device_condition_event(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_device_condition_event(&self.0)
    }

    /// GetDeviceCondition \[1.0.0-5.0.2\] (cmd 3).
    #[inline]
    pub fn get_device_condition_v100(
        &self,
        out: &mut BtmDeviceConditionV100,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_device_condition_v100(&self.0, out)
    }

    /// GetDeviceCondition \[5.1.0-7.0.1\] (cmd 3).
    #[inline]
    pub fn get_device_condition_v510(
        &self,
        out: &mut BtmDeviceConditionV510,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_device_condition_v510(&self.0, out)
    }

    /// GetDeviceCondition \[8.0.0-8.1.1\] (cmd 3).
    #[inline]
    pub fn get_device_condition_v800(
        &self,
        out: &mut BtmDeviceConditionV800,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_device_condition_v800(&self.0, out)
    }

    /// GetDeviceCondition \[9.0.0-12.1.0\] (cmd 3).
    #[inline]
    pub fn get_device_condition_v900(
        &self,
        out: &mut BtmDeviceConditionV900,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_device_condition_v900(&self.0, out)
    }

    /// GetDeviceCondition \[13.0.0+\] (cmd 3).
    #[inline]
    pub fn get_device_condition(
        &self,
        profile: BtmProfile,
        out: &mut [BtmConnectedDeviceV13],
    ) -> Result<i32, nx_sf::service::DispatchError> {
        cmif::get_device_condition(&self.0, profile as u32, out)
    }

    /// SetBurstMode (cmd 4).
    #[inline]
    pub fn set_burst_mode(
        &self,
        addr: &BtdrvAddress,
        flag: bool,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_burst_mode(&self.0, addr, flag)
    }

    /// SetSlotMode (cmd 5).
    #[inline]
    pub fn set_slot_mode(
        &self,
        list: &BtmDeviceSlotModeList,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_slot_mode(&self.0, list)
    }

    /// SetBluetoothMode (cmd 6, pre-9.0.0).
    #[inline]
    pub fn set_bluetooth_mode(
        &self,
        mode: BtmBluetoothMode,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_bluetooth_mode(&self.0, mode as u32)
    }

    /// SetWlanMode (cmd 7).
    #[inline]
    pub fn set_wlan_mode(&self, mode: BtmWlanMode) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_wlan_mode(&self.0, mode as u32)
    }

    /// AcquireDeviceInfoEvent (cmd 8, pre-3.0.0).
    #[inline]
    pub fn acquire_device_info_event_legacy(&self) -> Result<u32, AcquireEventError> {
        cmif::acquire_device_info_event_legacy(&self.0)
    }

    /// AcquireDeviceInfoEvent (cmd 8, 3.0.0+).
    #[inline]
    pub fn acquire_device_info_event(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_device_info_event(&self.0)
    }

    /// GetDeviceInfo \[1.0.0-12.1.0\] (cmd 9).
    #[inline]
    pub fn get_device_info_legacy(
        &self,
        out: &mut BtmDeviceInfoList,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::get_device_info_legacy(&self.0, out)
    }

    /// GetDeviceInfo \[13.0.0+\] (cmd 9).
    #[inline]
    pub fn get_device_info(
        &self,
        profile: BtmProfile,
        out: &mut [BtmDeviceInfoV13],
    ) -> Result<i32, nx_sf::service::DispatchError> {
        cmif::get_device_info(&self.0, profile as u32, out)
    }

    /// AddDeviceInfo \[1.0.0-12.1.0\] (cmd 10).
    #[inline]
    pub fn add_device_info_legacy(
        &self,
        info: &BtmDeviceInfoV1,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::add_device_info_legacy(&self.0, info)
    }

    /// AddDeviceInfo \[13.0.0+\] (cmd 10).
    #[inline]
    pub fn add_device_info(
        &self,
        info: &BtmDeviceInfoV13,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::add_device_info(&self.0, info)
    }

    /// RemoveDeviceInfo (cmd 11).
    #[inline]
    pub fn remove_device_info(
        &self,
        addr: &BtdrvAddress,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::remove_device_info(&self.0, addr)
    }

    /// IncreaseDeviceInfoOrder (cmd 12).
    #[inline]
    pub fn increase_device_info_order(
        &self,
        addr: &BtdrvAddress,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::increase_device_info_order(&self.0, addr)
    }

    /// LlrNotify \[pre-9.0.0\] (cmd 13).
    #[inline]
    pub fn llr_notify_legacy(
        &self,
        addr: &BtdrvAddress,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::llr_notify_legacy(&self.0, addr)
    }

    /// LlrNotify \[9.0.0+\] (cmd 13).
    #[inline]
    pub fn llr_notify(
        &self,
        addr: &BtdrvAddress,
        unk: i32,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::llr_notify(&self.0, addr, unk)
    }

    /// EnableRadio (cmd 14).
    #[inline]
    pub fn enable_radio(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::enable_radio(&self.0)
    }

    /// DisableRadio (cmd 15).
    #[inline]
    pub fn disable_radio(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::disable_radio(&self.0)
    }

    /// HidDisconnect (cmd 16).
    #[inline]
    pub fn hid_disconnect(&self, addr: &BtdrvAddress) -> Result<(), nx_sf::service::DispatchError> {
        cmif::hid_disconnect(&self.0, addr)
    }

    /// HidSetRetransmissionMode (cmd 17).
    #[inline]
    pub fn hid_set_retransmission_mode(
        &self,
        addr: &BtdrvAddress,
        list: &BtmZeroRetransmissionList,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::hid_set_retransmission_mode(&self.0, addr, list)
    }
}

// ---------------------------------------------------------------------------
// Extended commands (2.0.0+ / 4.0.0+ / 5.0.0+)
// ---------------------------------------------------------------------------

impl BtmService {
    /// AcquireAwakeReqEvent (cmd 18, pre-3.0.0, 2.0.0+).
    #[inline]
    pub fn acquire_awake_req_event_legacy(&self) -> Result<u32, AcquireEventError> {
        cmif::acquire_awake_req_event_legacy(&self.0)
    }

    /// AcquireAwakeReqEvent (cmd 18, 3.0.0+).
    #[inline]
    pub fn acquire_awake_req_event(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_awake_req_event(&self.0)
    }

    /// AcquireLlrStateEvent (cmd 19, 4.0.0+).
    #[inline]
    pub fn acquire_llr_state_event(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_llr_state_event(&self.0)
    }

    /// IsLlrStarted (cmd 20, 4.0.0+).
    #[inline]
    pub fn is_llr_started(&self) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::is_llr_started(&self.0)
    }

    /// EnableSlotSaving (cmd 21, 4.0.0+).
    #[inline]
    pub fn enable_slot_saving(&self, flag: bool) -> Result<(), nx_sf::service::DispatchError> {
        cmif::enable_slot_saving(&self.0, flag)
    }

    /// ProtectDeviceInfo (cmd 22, 5.0.0+).
    #[inline]
    pub fn protect_device_info(
        &self,
        addr: &BtdrvAddress,
        flag: bool,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::protect_device_info(&self.0, addr, flag)
    }
}

// ---------------------------------------------------------------------------
// BLE scan commands (5.0.0+/5.1.0+)
// ---------------------------------------------------------------------------

impl BtmService {
    /// AcquireBleScanEvent (cmd 23, 5.0.0+).
    #[inline]
    pub fn acquire_ble_scan_event(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_ble_scan_event(&self.0)
    }

    /// GetBleScanParameterGeneral (cmd 24, 5.1.0+).
    #[inline]
    pub fn get_ble_scan_parameter_general(
        &self,
        parameter_id: u16,
    ) -> Result<BtdrvBleAdvertisePacketParameter, nx_sf::service::DispatchError> {
        cmif::get_ble_scan_parameter_general(&self.0, parameter_id)
    }

    /// GetBleScanParameterSmartDevice (cmd 25, 5.1.0+).
    #[inline]
    pub fn get_ble_scan_parameter_smart_device(
        &self,
        parameter_id: u16,
    ) -> Result<BtdrvGattAttributeUuid, nx_sf::service::DispatchError> {
        cmif::get_ble_scan_parameter_smart_device(&self.0, parameter_id)
    }

    /// StartBleScanForGeneral (cmd 26, 5.1.0+).
    #[inline]
    pub fn start_ble_scan_for_general(
        &self,
        param: &BtdrvBleAdvertisePacketParameter,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::start_ble_scan_for_general(&self.0, param)
    }

    /// StopBleScanForGeneral (cmd 27, 5.1.0+).
    #[inline]
    pub fn stop_ble_scan_for_general(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::stop_ble_scan_for_general(&self.0)
    }

    /// GetBleScanResultsForGeneral (cmd 28, 5.1.0+).
    #[inline]
    pub fn get_ble_scan_results_for_general(
        &self,
        results: &mut [BtdrvBleScanResult],
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_ble_scan_results_for_general(&self.0, results)
    }

    /// StartBleScanForPaired (cmd 29, 5.1.0+).
    #[inline]
    pub fn start_ble_scan_for_paired(
        &self,
        param: &BtdrvBleAdvertisePacketParameter,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::start_ble_scan_for_paired(&self.0, param)
    }

    /// StopBleScanForPaired (cmd 30, 5.1.0+).
    #[inline]
    pub fn stop_ble_scan_for_paired(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::stop_ble_scan_for_paired(&self.0)
    }

    /// StartBleScanForSmartDevice (cmd 31, 5.1.0+).
    #[inline]
    pub fn start_ble_scan_for_smart_device(
        &self,
        uuid: &BtdrvGattAttributeUuid,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::start_ble_scan_for_smart_device(&self.0, uuid)
    }

    /// StopBleScanForSmartDevice (cmd 32, 5.1.0+).
    #[inline]
    pub fn stop_ble_scan_for_smart_device(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::stop_ble_scan_for_smart_device(&self.0)
    }

    /// GetBleScanResultsForSmartDevice (cmd 33, 5.1.0+).
    #[inline]
    pub fn get_ble_scan_results_for_smart_device(
        &self,
        results: &mut [BtdrvBleScanResult],
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_ble_scan_results_for_smart_device(&self.0, results)
    }
}

// ---------------------------------------------------------------------------
// BLE connection commands (5.0.0+/5.1.0+)
// ---------------------------------------------------------------------------

impl BtmService {
    /// AcquireBleConnectionEvent (cmd 34, 5.1.0+).
    #[inline]
    pub fn acquire_ble_connection_event(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_ble_connection_event(&self.0)
    }

    /// BleConnect (cmd 35, 5.1.0+).
    #[inline]
    pub fn ble_connect(&self, addr: &BtdrvAddress) -> Result<(), nx_sf::service::DispatchError> {
        cmif::ble_connect(&self.0, addr)
    }

    /// BleConnect \[5.0.0-5.0.2\] (cmd 24).
    #[inline]
    pub fn ble_connect_legacy(
        &self,
        addr: &BtdrvAddress,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::ble_connect_legacy(&self.0, addr)
    }

    /// BleOverrideConnection (cmd 36, 5.1.0+).
    #[inline]
    pub fn ble_override_connection(&self, id: u32) -> Result<(), nx_sf::service::DispatchError> {
        cmif::ble_override_connection(&self.0, id)
    }

    /// BleDisconnect (cmd 37, 5.1.0+).
    #[inline]
    pub fn ble_disconnect(
        &self,
        connection_handle: u32,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::ble_disconnect(&self.0, connection_handle)
    }

    /// BleDisconnect \[5.0.0-5.0.2\] (cmd 25).
    #[inline]
    pub fn ble_disconnect_legacy(
        &self,
        connection_handle: u32,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::ble_disconnect_legacy(&self.0, connection_handle)
    }

    /// BleGetConnectionState (cmd 38, 5.1.0+).
    #[inline]
    pub fn ble_get_connection_state(
        &self,
        info: &mut [BtdrvBleConnectionInfo],
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::ble_get_connection_state(&self.0, info)
    }

    /// BleGetConnectionState \[5.0.0-5.0.2\] (cmd 26).
    #[inline]
    pub fn ble_get_connection_state_legacy(
        &self,
        info: &mut [BtdrvBleConnectionInfo],
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::ble_get_connection_state_legacy(&self.0, info)
    }

    /// BleGetGattClientConditionList (cmd 39, 5.1.0+).
    #[inline]
    pub fn ble_get_gatt_client_condition_list(
        &self,
        list: &mut BtmGattClientConditionList,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::ble_get_gatt_client_condition_list(&self.0, list)
    }

    /// BleGetGattClientConditionList \[5.0.0-5.0.2\] (cmd 27).
    #[inline]
    pub fn ble_get_gatt_client_condition_list_legacy(
        &self,
        list: &mut BtmGattClientConditionList,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::ble_get_gatt_client_condition_list_legacy(&self.0, list)
    }
}

// ---------------------------------------------------------------------------
// BLE pairing commands (5.0.0+/5.1.0+)
// ---------------------------------------------------------------------------

impl BtmService {
    /// AcquireBlePairingEvent (cmd 40, 5.1.0+).
    #[inline]
    pub fn acquire_ble_pairing_event(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_ble_pairing_event(&self.0)
    }

    /// AcquireBlePairingEvent \[5.0.0-5.0.2\] (cmd 28).
    #[inline]
    pub fn acquire_ble_pairing_event_legacy(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_ble_pairing_event_legacy(&self.0)
    }

    /// BlePairDevice (cmd 41, 5.1.0+).
    #[inline]
    pub fn ble_pair_device(
        &self,
        connection_handle: u32,
        param: &BtdrvBleAdvertisePacketParameter,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::ble_pair_device(&self.0, connection_handle, param)
    }

    /// BleUnpairDeviceOnBoth (cmd 42, 5.1.0+).
    #[inline]
    pub fn ble_unpair_device_on_both(
        &self,
        connection_handle: u32,
        param: &BtdrvBleAdvertisePacketParameter,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::ble_unpair_device_on_both(&self.0, connection_handle, param)
    }

    /// BleUnPairDevice (cmd 43, 5.1.0+).
    #[inline]
    pub fn ble_unpair_device(
        &self,
        addr: &BtdrvAddress,
        param: &BtdrvBleAdvertisePacketParameter,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::ble_unpair_device(&self.0, addr, param)
    }

    /// BleGetPairedAddresses (cmd 44, 5.1.0+).
    #[inline]
    pub fn ble_get_paired_addresses(
        &self,
        param: &BtdrvBleAdvertisePacketParameter,
        addrs: &mut [BtdrvAddress],
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::ble_get_paired_addresses(&self.0, param, addrs)
    }
}

// ---------------------------------------------------------------------------
// GATT service discovery commands (5.0.0+/5.1.0+)
// ---------------------------------------------------------------------------

impl BtmService {
    /// AcquireBleServiceDiscoveryEvent (cmd 45, 5.1.0+).
    #[inline]
    pub fn acquire_ble_service_discovery_event(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_ble_service_discovery_event(&self.0)
    }

    /// GetGattServices (cmd 46, 5.1.0+).
    #[inline]
    pub fn get_gatt_services(
        &self,
        connection_handle: u32,
        services: &mut [BtmGattService],
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_gatt_services(&self.0, connection_handle, services)
    }

    /// GetGattServices \[5.0.0-5.0.2\] (cmd 29).
    #[inline]
    pub fn get_gatt_services_legacy(
        &self,
        connection_handle: u32,
        services: &mut [BtmGattService],
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_gatt_services_legacy(&self.0, connection_handle, services)
    }

    /// GetGattService (cmd 47, 5.1.0+).
    #[inline]
    pub fn get_gatt_service(
        &self,
        connection_handle: u32,
        uuid: &BtdrvGattAttributeUuid,
        out_service: &mut BtmGattService,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::get_gatt_service(&self.0, connection_handle, uuid, out_service)
    }

    /// GetGattService \[5.0.0-5.0.2\] (cmd 30).
    #[inline]
    pub fn get_gatt_service_legacy(
        &self,
        connection_handle: u32,
        uuid: &BtdrvGattAttributeUuid,
        out_service: &mut BtmGattService,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::get_gatt_service_legacy(&self.0, connection_handle, uuid, out_service)
    }

    /// GetGattIncludedServices (cmd 48, 5.1.0+).
    #[inline]
    pub fn get_gatt_included_services(
        &self,
        connection_handle: u32,
        service_handle: u16,
        services: &mut [BtmGattService],
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_gatt_included_services(&self.0, connection_handle, service_handle, services)
    }

    /// GetGattIncludedServices \[5.0.0-5.0.2\] (cmd 31).
    #[inline]
    pub fn get_gatt_included_services_legacy(
        &self,
        connection_handle: u32,
        service_handle: u16,
        services: &mut [BtmGattService],
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_gatt_included_services_legacy(
            &self.0,
            connection_handle,
            service_handle,
            services,
        )
    }

    /// GetBelongingService (cmd 49, 5.1.0+).
    #[inline]
    pub fn get_belonging_service(
        &self,
        connection_handle: u32,
        attribute_handle: u16,
        out_service: &mut BtmGattService,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::get_belonging_service(&self.0, connection_handle, attribute_handle, out_service)
    }

    /// GetBelongingService \[5.0.0-5.0.2\] (cmd 32).
    #[inline]
    pub fn get_belonging_service_legacy(
        &self,
        connection_handle: u32,
        attribute_handle: u16,
        out_service: &mut BtmGattService,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::get_belonging_service_legacy(
            &self.0,
            connection_handle,
            attribute_handle,
            out_service,
        )
    }

    /// GetGattCharacteristics (cmd 50, 5.1.0+).
    #[inline]
    pub fn get_gatt_characteristics(
        &self,
        connection_handle: u32,
        service_handle: u16,
        characteristics: &mut [BtmGattCharacteristic],
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_gatt_characteristics(&self.0, connection_handle, service_handle, characteristics)
    }

    /// GetGattCharacteristics \[5.0.0-5.0.2\] (cmd 33).
    #[inline]
    pub fn get_gatt_characteristics_legacy(
        &self,
        connection_handle: u32,
        service_handle: u16,
        characteristics: &mut [BtmGattCharacteristic],
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_gatt_characteristics_legacy(
            &self.0,
            connection_handle,
            service_handle,
            characteristics,
        )
    }

    /// GetGattDescriptors (cmd 51, 5.1.0+).
    #[inline]
    pub fn get_gatt_descriptors(
        &self,
        connection_handle: u32,
        char_handle: u16,
        descriptors: &mut [BtmGattDescriptor],
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_gatt_descriptors(&self.0, connection_handle, char_handle, descriptors)
    }

    /// GetGattDescriptors \[5.0.0-5.0.2\] (cmd 34).
    #[inline]
    pub fn get_gatt_descriptors_legacy(
        &self,
        connection_handle: u32,
        char_handle: u16,
        descriptors: &mut [BtmGattDescriptor],
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_gatt_descriptors_legacy(&self.0, connection_handle, char_handle, descriptors)
    }
}

// ---------------------------------------------------------------------------
// BLE MTU commands (5.0.0+/5.1.0+)
// ---------------------------------------------------------------------------

impl BtmService {
    /// AcquireBleMtuConfigEvent (cmd 52, 5.1.0+).
    #[inline]
    pub fn acquire_ble_mtu_config_event(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_ble_mtu_config_event(&self.0)
    }

    /// AcquireBleMtuConfigEvent \[5.0.0-5.0.2\] (cmd 35).
    #[inline]
    pub fn acquire_ble_mtu_config_event_legacy(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_ble_mtu_config_event_legacy(&self.0)
    }

    /// ConfigureBleMtu (cmd 53, 5.1.0+).
    #[inline]
    pub fn configure_ble_mtu(
        &self,
        connection_handle: u32,
        mtu: u16,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::configure_ble_mtu(&self.0, connection_handle, mtu)
    }

    /// ConfigureBleMtu \[5.0.0-5.0.2\] (cmd 36).
    #[inline]
    pub fn configure_ble_mtu_legacy(
        &self,
        connection_handle: u32,
        mtu: u16,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::configure_ble_mtu_legacy(&self.0, connection_handle, mtu)
    }

    /// GetBleMtu (cmd 54, 5.1.0+).
    #[inline]
    pub fn get_ble_mtu(
        &self,
        connection_handle: u32,
    ) -> Result<u16, nx_sf::service::DispatchError> {
        cmif::get_ble_mtu(&self.0, connection_handle)
    }

    /// GetBleMtu \[5.0.0-5.0.2\] (cmd 37).
    #[inline]
    pub fn get_ble_mtu_legacy(
        &self,
        connection_handle: u32,
    ) -> Result<u16, nx_sf::service::DispatchError> {
        cmif::get_ble_mtu_legacy(&self.0, connection_handle)
    }
}

// ---------------------------------------------------------------------------
// GATT data path commands (5.0.0+/5.1.0+)
// ---------------------------------------------------------------------------

impl BtmService {
    /// RegisterBleGattDataPath (cmd 55, 5.1.0+).
    #[inline]
    pub fn register_ble_gatt_data_path(
        &self,
        path: &BtmBleDataPath,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::register_ble_gatt_data_path(&self.0, path)
    }

    /// RegisterBleGattDataPath \[5.0.0-5.0.2\] (cmd 38).
    #[inline]
    pub fn register_ble_gatt_data_path_legacy(
        &self,
        path: &BtmBleDataPath,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::register_ble_gatt_data_path_legacy(&self.0, path)
    }

    /// UnregisterBleGattDataPath (cmd 56, 5.1.0+).
    #[inline]
    pub fn unregister_ble_gatt_data_path(
        &self,
        path: &BtmBleDataPath,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::unregister_ble_gatt_data_path(&self.0, path)
    }

    /// UnregisterBleGattDataPath \[5.0.0-5.0.2\] (cmd 39).
    #[inline]
    pub fn unregister_ble_gatt_data_path_legacy(
        &self,
        path: &BtmBleDataPath,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::unregister_ble_gatt_data_path_legacy(&self.0, path)
    }
}

// ---------------------------------------------------------------------------
// Applet resource user ID commands (5.0.0+/5.1.0+)
// ---------------------------------------------------------------------------

impl BtmService {
    /// RegisterAppletResourceUserId (cmd 57, 5.1.0+).
    #[inline]
    pub fn register_applet_resource_user_id(
        &self,
        applet_resource_user_id: u64,
        unk: u32,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::register_applet_resource_user_id(&self.0, applet_resource_user_id, unk)
    }

    /// RegisterAppletResourceUserId \[5.0.0-5.0.2\] (cmd 40).
    #[inline]
    pub fn register_applet_resource_user_id_legacy(
        &self,
        applet_resource_user_id: u64,
        unk: u32,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::register_applet_resource_user_id_legacy(&self.0, applet_resource_user_id, unk)
    }

    /// UnregisterAppletResourceUserId (cmd 58, 5.1.0+).
    #[inline]
    pub fn unregister_applet_resource_user_id(
        &self,
        applet_resource_user_id: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::unregister_applet_resource_user_id(&self.0, applet_resource_user_id)
    }

    /// UnregisterAppletResourceUserId \[5.0.0-5.0.2\] (cmd 41).
    #[inline]
    pub fn unregister_applet_resource_user_id_legacy(
        &self,
        applet_resource_user_id: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::unregister_applet_resource_user_id_legacy(&self.0, applet_resource_user_id)
    }

    /// SetAppletResourceUserId (cmd 59, 5.1.0+).
    #[inline]
    pub fn set_applet_resource_user_id(
        &self,
        applet_resource_user_id: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_applet_resource_user_id(&self.0, applet_resource_user_id)
    }

    /// SetAppletResourceUserId \[5.0.0-5.0.2\] (cmd 42).
    #[inline]
    pub fn set_applet_resource_user_id_legacy(
        &self,
        applet_resource_user_id: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::set_applet_resource_user_id_legacy(&self.0, applet_resource_user_id)
    }
}

// ---------------------------------------------------------------------------
// Connection
// ---------------------------------------------------------------------------

/// Connects to the Bluetooth Manager service (`btm`) using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<BtmService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    Ok(BtmService(Session::new(handle, 0)))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    #[error("failed to get btm service")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
}
