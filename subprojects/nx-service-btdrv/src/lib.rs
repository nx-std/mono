//! Bluetooth Driver service (`btdrv`) implementation.
//!
//! Provides low-level Bluetooth driver access including classic Bluetooth,
//! HID, BLE, GATT, and audio streaming for the Nintendo Switch.
//!
//! This is a non-domain service. On initialization, cmd 0 (InitializeBluetoothDriver)
//! is called automatically.
//!
//! Many commands have hosversion-dependent wire formats or command IDs.
//! Per IC-4 (hosversion-unaware), paired `_legacy` / versioned method variants
//! are exposed and the caller selects based on the system version.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{DispatchError, Session};
use nx_svc::ipc::Handle;

mod cmif;
mod dispatch;
mod proto;
mod types;

use self::types::{
    AddGattCharacteristicIn, AddGattDescriptorIn, AddGattServiceIn, AddrU8In, AddrU32In,
    AddrU32U32In, CancelConnectGattServerIn, ConfigureAttMtuIn, ConnectGattClientIn,
    ConnectGattServerIn, DisconnectGattClientLegacyIn, EnableGattServiceIn, GattNotificationIn,
    GetGattAttributeLegacyIn, GetGattFirstCharacteristicIn, GetGattFirstDescriptorIn,
    GetGattNextCharacteristicIn, GetGattNextDescriptorIn, GetGattServiceIn, GetHidReportIn,
    LegacyRespondToPinRequestIn, ReadGattCharacteristicIn, ReadGattDescriptorIn,
    RespondToPinRequestIn, RespondToSspRequestIn, RespondToSspRequestLegacyIn,
    SetBleAdvertiseParameterIn, SetBleConnectionParameterIn, SetBleScanParameterIn, SetHidReportIn,
    StartAudioOutIn, StartInquiryIn, TriggerConnectionIn, TwoBoolsIn, WriteGattCharacteristicIn,
    WriteGattDescriptorIn,
};
pub use self::{
    cmif::AcquireEventError,
    proto::SERVICE_NAME,
    types::{
        BtdrvAdapterProperty, BtdrvAdapterPropertyOld, BtdrvAdapterPropertySet, BtdrvAddress,
        BtdrvAudioControlButtonState, BtdrvBleAdvertiseFilter, BtdrvBleAdvertisePacketData,
        BtdrvBleAdvertisement, BtdrvBleConnectionParameter, BtdrvBluetoothPinCode,
        BtdrvChannelMapList, BtdrvClassOfDevice, BtdrvFatalReason, BtdrvGattAttributeUuid,
        BtdrvGattId, BtdrvHidData, BtdrvHidReport, BtdrvLeConnectionParams, BtdrvPcmParameter,
        BtdrvPinCode, BtdrvPlrList, BtdrvPlrStatistics, SetSysBluetoothDevicesSettings,
    },
};

/// Bluetooth Driver service wrapper.
#[repr(transparent)]
pub struct BtdrvService(Session);

impl BtdrvService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> Handle {
        self.0.handle()
    }
}

// Core Bluetooth

impl BtdrvService {
    /// InitializeBluetooth (cmd 1). Returns event handle.
    #[inline]
    pub fn initialize_bluetooth(&self) -> Result<u32, AcquireEventError> {
        cmif::initialize_bluetooth(&self.0)
    }

    /// EnableBluetooth (cmd 2).
    #[inline]
    pub fn enable_bluetooth(&self) -> Result<(), DispatchError> {
        cmif::enable_bluetooth(&self.0)
    }

    /// DisableBluetooth (cmd 3).
    #[inline]
    pub fn disable_bluetooth(&self) -> Result<(), DispatchError> {
        cmif::disable_bluetooth(&self.0)
    }

    /// FinalizeBluetooth (cmd 4).
    #[inline]
    pub fn finalize_bluetooth(&self) -> Result<(), DispatchError> {
        cmif::finalize_bluetooth(&self.0)
    }

    /// GetAdapterProperties \[1.0.0-11.0.1\] (cmd 5).
    #[inline]
    pub fn get_adapter_properties_legacy(
        &self,
        out: &mut BtdrvAdapterPropertyOld,
    ) -> Result<(), DispatchError> {
        cmif::get_adapter_properties_legacy(&self.0, out)
    }

    /// GetAdapterProperties \[12.0.0+\] (cmd 5).
    #[inline]
    pub fn get_adapter_properties(
        &self,
        out: &mut BtdrvAdapterPropertySet,
    ) -> Result<(), DispatchError> {
        cmif::get_adapter_properties(&self.0, out)
    }

    /// GetAdapterProperty \[1.0.0-11.0.1\] (cmd 6).
    #[inline]
    pub fn get_adapter_property_legacy(
        &self,
        property_type: u32,
        buf: &mut [u8],
    ) -> Result<(), DispatchError> {
        cmif::get_adapter_property_legacy(&self.0, property_type, buf)
    }

    /// GetAdapterProperty \[12.0.0+\] (cmd 6).
    #[inline]
    pub fn get_adapter_property(
        &self,
        property_type: u32,
        out: &mut BtdrvAdapterProperty,
    ) -> Result<(), DispatchError> {
        cmif::get_adapter_property(&self.0, property_type, out)
    }

    /// SetAdapterProperty \[1.0.0-11.0.1\] (cmd 7).
    #[inline]
    pub fn set_adapter_property_legacy(
        &self,
        property_type: u32,
        buf: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::set_adapter_property_legacy(&self.0, property_type, buf)
    }

    /// SetAdapterProperty \[12.0.0+\] (cmd 7).
    #[inline]
    pub fn set_adapter_property(
        &self,
        property_type: u32,
        input: &BtdrvAdapterProperty,
    ) -> Result<(), DispatchError> {
        cmif::set_adapter_property(&self.0, property_type, input)
    }

    /// StartInquiry \[1.0.0-11.0.1\] (cmd 8).
    #[inline]
    pub fn start_inquiry_legacy(&self) -> Result<(), DispatchError> {
        cmif::start_inquiry_legacy(&self.0)
    }

    /// StartInquiry \[12.0.0+\] (cmd 8).
    #[inline]
    pub fn start_inquiry(&self, services: u32, duration: i64) -> Result<(), DispatchError> {
        cmif::start_inquiry(
            &self.0,
            StartInquiryIn {
                services,
                _pad: 0,
                duration,
            },
        )
    }

    /// StopInquiry (cmd 9).
    #[inline]
    pub fn stop_inquiry(&self) -> Result<(), DispatchError> {
        cmif::stop_inquiry(&self.0)
    }

    /// CreateBond \[1.0.0-8.1.1\] (cmd 10).
    #[inline]
    pub fn create_bond_legacy(
        &self,
        addr: BtdrvAddress,
        bond_type: u32,
    ) -> Result<(), DispatchError> {
        cmif::create_bond_legacy(&self.0, addr, bond_type)
    }

    /// CreateBond \[9.0.0+\] (cmd 10).
    #[inline]
    pub fn create_bond(&self, addr: BtdrvAddress, bond_type: u32) -> Result<(), DispatchError> {
        cmif::create_bond(
            &self.0,
            AddrU32In {
                addr,
                pad: [0; 2],
                val: bond_type,
            },
        )
    }

    /// RemoveBond (cmd 11).
    #[inline]
    pub fn remove_bond(&self, addr: BtdrvAddress) -> Result<(), DispatchError> {
        cmif::remove_bond(&self.0, addr)
    }

    /// CancelBond (cmd 12).
    #[inline]
    pub fn cancel_bond(&self, addr: BtdrvAddress) -> Result<(), DispatchError> {
        cmif::cancel_bond(&self.0, addr)
    }

    /// RespondToPinRequest \[1.0.0-11.0.1\] (cmd 13).
    #[inline]
    pub fn respond_to_pin_request_legacy(
        &self,
        addr: BtdrvAddress,
        flag: bool,
        pin_code: &BtdrvBluetoothPinCode,
        length: u8,
    ) -> Result<(), DispatchError> {
        cmif::respond_to_pin_request_legacy(
            &self.0,
            LegacyRespondToPinRequestIn {
                addr,
                flag: flag as u8,
                length,
                pin_code: *pin_code,
            },
        )
    }

    /// RespondToPinRequest \[12.0.0+\] (cmd 13).
    #[inline]
    pub fn respond_to_pin_request(
        &self,
        addr: BtdrvAddress,
        pin_code: &BtdrvPinCode,
    ) -> Result<(), DispatchError> {
        cmif::respond_to_pin_request(
            &self.0,
            RespondToPinRequestIn {
                addr,
                pin_code: *pin_code,
            },
        )
    }

    /// RespondToSspRequest \[1.0.0-11.0.1\] (cmd 14).
    #[inline]
    pub fn respond_to_ssp_request_legacy(
        &self,
        addr: BtdrvAddress,
        variant: u32,
        accept: bool,
        passkey: u32,
    ) -> Result<(), DispatchError> {
        cmif::respond_to_ssp_request_legacy(
            &self.0,
            RespondToSspRequestLegacyIn {
                addr,
                variant: variant as u8,
                accept: accept as u8,
                passkey,
            },
        )
    }

    /// RespondToSspRequest \[12.0.0+\] (cmd 14).
    #[inline]
    pub fn respond_to_ssp_request(
        &self,
        addr: BtdrvAddress,
        variant: u32,
        accept: bool,
        passkey: u32,
    ) -> Result<(), DispatchError> {
        cmif::respond_to_ssp_request(
            &self.0,
            RespondToSspRequestIn {
                addr,
                accept: accept as u8,
                _pad: 0,
                variant,
                passkey,
            },
        )
    }

    /// GetEventInfo (cmd 15). Returns event type.
    #[inline]
    pub fn get_event_info(&self, buf: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::get_event_info(&self.0, buf)
    }
}

// HID

impl BtdrvService {
    /// InitializeHid (cmd 16). Returns event handle.
    #[inline]
    pub fn initialize_hid(&self) -> Result<u32, AcquireEventError> {
        cmif::initialize_hid(&self.0)
    }

    /// OpenHidConnection (cmd 17).
    #[inline]
    pub fn open_hid_connection(&self, addr: BtdrvAddress) -> Result<(), DispatchError> {
        cmif::open_hid_connection(&self.0, addr)
    }

    /// CloseHidConnection (cmd 18).
    #[inline]
    pub fn close_hid_connection(&self, addr: BtdrvAddress) -> Result<(), DispatchError> {
        cmif::close_hid_connection(&self.0, addr)
    }

    /// WriteHidData \[1.0.0-8.1.1\] (cmd 19).
    #[inline]
    pub fn write_hid_data_legacy(
        &self,
        addr: BtdrvAddress,
        report: &BtdrvHidData,
    ) -> Result<(), DispatchError> {
        cmif::write_hid_data_legacy(&self.0, addr, report)
    }

    /// WriteHidData \[9.0.0+\] (cmd 19).
    #[inline]
    pub fn write_hid_data(
        &self,
        addr: BtdrvAddress,
        report: &BtdrvHidReport,
    ) -> Result<(), DispatchError> {
        cmif::write_hid_data(&self.0, addr, report)
    }

    /// WriteHidData2 (cmd 20).
    #[inline]
    pub fn write_hid_data2(&self, addr: BtdrvAddress, buf: &[u8]) -> Result<(), DispatchError> {
        cmif::write_hid_data2(&self.0, addr, buf)
    }

    /// SetHidReport \[1.0.0-8.1.1\] (cmd 21).
    #[inline]
    pub fn set_hid_report_legacy(
        &self,
        addr: BtdrvAddress,
        report_type: u32,
        report: &BtdrvHidData,
    ) -> Result<(), DispatchError> {
        cmif::set_hid_report_legacy(
            &self.0,
            SetHidReportIn {
                addr,
                pad: [0; 2],
                report_type,
            },
            report,
        )
    }

    /// SetHidReport \[9.0.0+\] (cmd 21).
    #[inline]
    pub fn set_hid_report(
        &self,
        addr: BtdrvAddress,
        report_type: u32,
        report: &BtdrvHidReport,
    ) -> Result<(), DispatchError> {
        cmif::set_hid_report(
            &self.0,
            SetHidReportIn {
                addr,
                pad: [0; 2],
                report_type,
            },
            report,
        )
    }

    /// GetHidReport (cmd 22).
    #[inline]
    pub fn get_hid_report(
        &self,
        addr: BtdrvAddress,
        report_id: u8,
        report_type: u32,
    ) -> Result<(), DispatchError> {
        cmif::get_hid_report(
            &self.0,
            GetHidReportIn {
                addr,
                report_id,
                pad: 0,
                report_type,
            },
        )
    }

    /// TriggerConnection \[1.0.0-8.1.1\] (cmd 23).
    #[inline]
    pub fn trigger_connection_legacy(&self, addr: BtdrvAddress) -> Result<(), DispatchError> {
        cmif::trigger_connection_legacy(&self.0, addr)
    }

    /// TriggerConnection \[9.0.0+\] (cmd 23).
    #[inline]
    pub fn trigger_connection(
        &self,
        addr: BtdrvAddress,
        timeout: u16,
    ) -> Result<(), DispatchError> {
        cmif::trigger_connection(&self.0, TriggerConnectionIn { addr, timeout })
    }

    /// AddPairedDeviceInfo (cmd 24).
    #[inline]
    pub fn add_paired_device_info(
        &self,
        settings: &SetSysBluetoothDevicesSettings,
    ) -> Result<(), DispatchError> {
        cmif::add_paired_device_info(&self.0, settings)
    }

    /// GetPairedDeviceInfo (cmd 25).
    #[inline]
    pub fn get_paired_device_info(
        &self,
        addr: BtdrvAddress,
        out: &mut SetSysBluetoothDevicesSettings,
    ) -> Result<(), DispatchError> {
        cmif::get_paired_device_info(&self.0, addr, out)
    }

    /// FinalizeHid (cmd 26).
    #[inline]
    pub fn finalize_hid(&self) -> Result<(), DispatchError> {
        cmif::finalize_hid(&self.0)
    }

    /// GetHidEventInfo (cmd 27). Returns event type.
    #[inline]
    pub fn get_hid_event_info(&self, buf: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::get_hid_event_info(&self.0, buf)
    }
}

// Radio/Modulation

impl BtdrvService {
    /// SetTsi (cmd 28).
    #[inline]
    pub fn set_tsi(&self, addr: BtdrvAddress, val: u8) -> Result<(), DispatchError> {
        cmif::set_tsi(&self.0, AddrU8In { addr, val })
    }

    /// EnableBurstMode (cmd 29).
    #[inline]
    pub fn enable_burst_mode(&self, addr: BtdrvAddress, flag: bool) -> Result<(), DispatchError> {
        cmif::enable_burst_mode(
            &self.0,
            AddrU8In {
                addr,
                val: flag as u8,
            },
        )
    }

    /// SetZeroRetransmission (cmd 30).
    #[inline]
    pub fn set_zero_retransmission(
        &self,
        addr: BtdrvAddress,
        report_ids: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::set_zero_retransmission(&self.0, addr, report_ids)
    }

    /// EnableMcMode (cmd 31).
    #[inline]
    pub fn enable_mc_mode(&self, flag: bool) -> Result<(), DispatchError> {
        cmif::enable_mc_mode(&self.0, flag as u8)
    }

    /// EnableLlrScan (cmd 32).
    #[inline]
    pub fn enable_llr_scan(&self) -> Result<(), DispatchError> {
        cmif::enable_llr_scan(&self.0)
    }

    /// DisableLlrScan (cmd 33).
    #[inline]
    pub fn disable_llr_scan(&self) -> Result<(), DispatchError> {
        cmif::disable_llr_scan(&self.0)
    }

    /// EnableRadio (cmd 34).
    #[inline]
    pub fn enable_radio(&self, flag: bool) -> Result<(), DispatchError> {
        cmif::enable_radio(&self.0, flag as u8)
    }

    /// SetVisibility (cmd 35).
    #[inline]
    pub fn set_visibility(
        &self,
        discoverable: bool,
        connectable: bool,
    ) -> Result<(), DispatchError> {
        cmif::set_visibility(
            &self.0,
            TwoBoolsIn {
                flag0: discoverable as u8,
                flag1: connectable as u8,
            },
        )
    }
}

// 4.0.0+ shifted commands

impl BtdrvService {
    /// EnableTbfcScan (cmd 36, 4.0.0+).
    #[inline]
    pub fn enable_tbfc_scan(&self, flag: bool) -> Result<(), DispatchError> {
        cmif::enable_tbfc_scan(&self.0, flag as u8)
    }

    /// RegisterHidReportEvent (cmd 37, 4.0.0+). Returns event handle.
    #[inline]
    pub fn register_hid_report_event(&self) -> Result<u32, AcquireEventError> {
        cmif::register_hid_report_event(&self.0)
    }

    /// RegisterHidReportEvent \[pre-4.0.0\] (cmd 36). Returns event handle.
    #[inline]
    pub fn register_hid_report_event_legacy(&self) -> Result<u32, AcquireEventError> {
        cmif::register_hid_report_event_legacy(&self.0)
    }

    /// GetHidReportEventInfo (cmd 38, 4.0.0+). Returns event type.
    #[inline]
    pub fn get_hid_report_event_info(&self, buf: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::get_hid_report_event_info(&self.0, buf)
    }

    /// GetHidReportEventInfo \[pre-4.0.0\] (cmd 37). Returns event type.
    #[inline]
    pub fn get_hid_report_event_info_legacy(&self, buf: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::get_hid_report_event_info_legacy(&self.0, buf)
    }

    /// GetHidReportEventInfo shared memory handle (cmd 38, 7.0.0+).
    /// Returns shared memory handle.
    #[inline]
    pub fn get_hid_report_event_shared_mem_handle(&self) -> Result<u32, AcquireEventError> {
        cmif::get_hid_report_event_shared_mem_handle(&self.0)
    }

    /// GetLatestPlr \[pre-9.0.0\] (4.0.0+, cmd 39).
    #[inline]
    pub fn get_latest_plr_statistics(
        &self,
        out: &mut BtdrvPlrStatistics,
    ) -> Result<(), DispatchError> {
        cmif::get_latest_plr_statistics(&self.0, out, proto::GET_LATEST_PLR)
    }

    /// GetLatestPlr \[pre-9.0.0, pre-4.0.0\] (cmd 38).
    #[inline]
    pub fn get_latest_plr_statistics_legacy(
        &self,
        out: &mut BtdrvPlrStatistics,
    ) -> Result<(), DispatchError> {
        cmif::get_latest_plr_statistics(&self.0, out, proto::GET_LATEST_PLR_LEGACY)
    }

    /// GetLatestPlr \[9.0.0+\] (4.0.0+, cmd 39).
    #[inline]
    pub fn get_latest_plr_list(&self, out: &mut BtdrvPlrList) -> Result<(), DispatchError> {
        cmif::get_latest_plr_list(&self.0, out, proto::GET_LATEST_PLR)
    }

    /// GetLatestPlr \[9.0.0+, pre-4.0.0\] (cmd 38).
    #[inline]
    pub fn get_latest_plr_list_legacy(&self, out: &mut BtdrvPlrList) -> Result<(), DispatchError> {
        cmif::get_latest_plr_list(&self.0, out, proto::GET_LATEST_PLR_LEGACY)
    }

    /// GetPendingConnections (cmd 40, 4.0.0+).
    #[inline]
    pub fn get_pending_connections(&self) -> Result<(), DispatchError> {
        cmif::get_pending_connections(&self.0, proto::GET_PENDING_CONNECTIONS)
    }

    /// GetPendingConnections \[pre-4.0.0\] (cmd 39).
    #[inline]
    pub fn get_pending_connections_legacy(&self) -> Result<(), DispatchError> {
        cmif::get_pending_connections(&self.0, proto::GET_PENDING_CONNECTIONS_LEGACY)
    }

    /// GetChannelMap (cmd 41, 4.0.0+).
    #[inline]
    pub fn get_channel_map(&self, out: &mut BtdrvChannelMapList) -> Result<(), DispatchError> {
        cmif::get_channel_map(&self.0, out, proto::GET_CHANNEL_MAP)
    }

    /// GetChannelMap \[pre-4.0.0\] (cmd 40).
    #[inline]
    pub fn get_channel_map_legacy(
        &self,
        out: &mut BtdrvChannelMapList,
    ) -> Result<(), DispatchError> {
        cmif::get_channel_map(&self.0, out, proto::GET_CHANNEL_MAP_LEGACY)
    }

    /// EnableTxPowerBoostSetting (cmd 42, 4.0.0+).
    #[inline]
    pub fn enable_tx_power_boost_setting(&self, flag: bool) -> Result<(), DispatchError> {
        cmif::enable_tx_power_boost_setting(
            &self.0,
            flag as u8,
            proto::ENABLE_TX_POWER_BOOST_SETTING,
        )
    }

    /// EnableTxPowerBoostSetting \[pre-4.0.0\] (cmd 41).
    #[inline]
    pub fn enable_tx_power_boost_setting_legacy(&self, flag: bool) -> Result<(), DispatchError> {
        cmif::enable_tx_power_boost_setting(
            &self.0,
            flag as u8,
            proto::ENABLE_TX_POWER_BOOST_SETTING_LEGACY,
        )
    }

    /// IsTxPowerBoostSettingEnabled (cmd 43, 4.0.0+).
    #[inline]
    pub fn is_tx_power_boost_setting_enabled(&self) -> Result<u8, DispatchError> {
        cmif::is_tx_power_boost_setting_enabled(&self.0, proto::IS_TX_POWER_BOOST_SETTING_ENABLED)
    }

    /// IsTxPowerBoostSettingEnabled \[pre-4.0.0\] (cmd 42).
    #[inline]
    pub fn is_tx_power_boost_setting_enabled_legacy(&self) -> Result<u8, DispatchError> {
        cmif::is_tx_power_boost_setting_enabled(
            &self.0,
            proto::IS_TX_POWER_BOOST_SETTING_ENABLED_LEGACY,
        )
    }

    /// EnableAfhSetting (cmd 44, 4.0.0+).
    #[inline]
    pub fn enable_afh_setting(&self, flag: bool) -> Result<(), DispatchError> {
        cmif::enable_afh_setting(&self.0, flag as u8, proto::ENABLE_AFH_SETTING)
    }

    /// EnableAfhSetting \[pre-4.0.0\] (cmd 43).
    #[inline]
    pub fn enable_afh_setting_legacy(&self, flag: bool) -> Result<(), DispatchError> {
        cmif::enable_afh_setting(&self.0, flag as u8, proto::ENABLE_AFH_SETTING_LEGACY)
    }

    /// IsAfhSettingEnabled (cmd 45, 4.0.0+).
    #[inline]
    pub fn is_afh_setting_enabled(&self) -> Result<u8, DispatchError> {
        cmif::is_afh_setting_enabled(&self.0, proto::IS_AFH_SETTING_ENABLED)
    }

    /// IsAfhSettingEnabled \[pre-4.0.0\] (cmd 44).
    #[inline]
    pub fn is_afh_setting_enabled_legacy(&self) -> Result<u8, DispatchError> {
        cmif::is_afh_setting_enabled(&self.0, proto::IS_AFH_SETTING_ENABLED_LEGACY)
    }
}

// BLE

impl BtdrvService {
    /// InitializeBle (cmd 46). Returns event handle.
    #[inline]
    pub fn initialize_ble(&self) -> Result<u32, AcquireEventError> {
        cmif::initialize_ble(&self.0)
    }

    /// EnableBle (cmd 47).
    #[inline]
    pub fn enable_ble(&self) -> Result<(), DispatchError> {
        cmif::enable_ble(&self.0)
    }

    /// DisableBle (cmd 48).
    #[inline]
    pub fn disable_ble(&self) -> Result<(), DispatchError> {
        cmif::disable_ble(&self.0)
    }

    /// FinalizeBle (cmd 49).
    #[inline]
    pub fn finalize_ble(&self) -> Result<(), DispatchError> {
        cmif::finalize_ble(&self.0)
    }

    /// SetBleVisibility (cmd 50).
    #[inline]
    pub fn set_ble_visibility(
        &self,
        discoverable: bool,
        connectable: bool,
    ) -> Result<(), DispatchError> {
        cmif::set_ble_visibility(
            &self.0,
            TwoBoolsIn {
                flag0: discoverable as u8,
                flag1: connectable as u8,
            },
        )
    }

    /// SetBleConnectionParameter \[5.0.0-8.1.1\] (cmd 51).
    #[inline]
    pub fn set_le_connection_parameter(
        &self,
        param: &BtdrvLeConnectionParams,
    ) -> Result<(), DispatchError> {
        cmif::set_le_connection_parameter(&self.0, *param)
    }

    /// SetBleConnectionParameter \[9.0.0+\] (cmd 51).
    #[inline]
    pub fn set_ble_connection_parameter(
        &self,
        addr: BtdrvAddress,
        param: &BtdrvBleConnectionParameter,
        flag: bool,
    ) -> Result<(), DispatchError> {
        cmif::set_ble_connection_parameter(
            &self.0,
            SetBleConnectionParameterIn {
                addr,
                flag: flag as u8,
                pad: 0,
                param: *param,
            },
        )
    }

    /// SetBleDefaultConnectionParameter \[5.0.0-8.1.1\] (cmd 52).
    #[inline]
    pub fn set_le_default_connection_parameter(
        &self,
        param: &BtdrvLeConnectionParams,
    ) -> Result<(), DispatchError> {
        cmif::set_le_default_connection_parameter(&self.0, *param)
    }

    /// SetBleDefaultConnectionParameter \[9.0.0+\] (cmd 52).
    #[inline]
    pub fn set_ble_default_connection_parameter(
        &self,
        param: &BtdrvBleConnectionParameter,
    ) -> Result<(), DispatchError> {
        cmif::set_ble_default_connection_parameter(&self.0, *param)
    }

    /// SetBleAdvertiseData (cmd 53).
    #[inline]
    pub fn set_ble_advertise_data(
        &self,
        data: &BtdrvBleAdvertisePacketData,
    ) -> Result<(), DispatchError> {
        cmif::set_ble_advertise_data(&self.0, data)
    }

    /// SetBleAdvertiseParameter (cmd 54).
    #[inline]
    pub fn set_ble_advertise_parameter(
        &self,
        addr: BtdrvAddress,
        min_interval: u16,
        max_interval: u16,
    ) -> Result<(), DispatchError> {
        cmif::set_ble_advertise_parameter(
            &self.0,
            SetBleAdvertiseParameterIn {
                addr,
                min_interval,
                max_interval,
            },
        )
    }

    /// StartBleScan (cmd 55).
    #[inline]
    pub fn start_ble_scan(&self) -> Result<(), DispatchError> {
        cmif::start_ble_scan(&self.0)
    }

    /// StopBleScan (cmd 56).
    #[inline]
    pub fn stop_ble_scan(&self) -> Result<(), DispatchError> {
        cmif::stop_ble_scan(&self.0)
    }

    /// AddBleScanFilterCondition (cmd 57).
    #[inline]
    pub fn add_ble_scan_filter_condition(
        &self,
        filter: &BtdrvBleAdvertiseFilter,
    ) -> Result<(), DispatchError> {
        cmif::add_ble_scan_filter_condition(&self.0, filter)
    }

    /// DeleteBleScanFilterCondition (cmd 58).
    #[inline]
    pub fn delete_ble_scan_filter_condition(
        &self,
        filter: &BtdrvBleAdvertiseFilter,
    ) -> Result<(), DispatchError> {
        cmif::delete_ble_scan_filter_condition(&self.0, filter)
    }

    /// DeleteBleScanFilter (cmd 59).
    #[inline]
    pub fn delete_ble_scan_filter(&self, index: u8) -> Result<(), DispatchError> {
        cmif::delete_ble_scan_filter(&self.0, index)
    }

    /// ClearBleScanFilters (cmd 60).
    #[inline]
    pub fn clear_ble_scan_filters(&self) -> Result<(), DispatchError> {
        cmif::clear_ble_scan_filters(&self.0)
    }

    /// EnableBleScanFilter (cmd 61).
    #[inline]
    pub fn enable_ble_scan_filter(&self, flag: bool) -> Result<(), DispatchError> {
        cmif::enable_ble_scan_filter(&self.0, flag as u8)
    }
}

// GATT

impl BtdrvService {
    /// RegisterGattClient (cmd 62).
    #[inline]
    pub fn register_gatt_client(&self, uuid: &BtdrvGattAttributeUuid) -> Result<(), DispatchError> {
        cmif::register_gatt_client(&self.0, *uuid)
    }

    /// UnregisterGattClient (cmd 63).
    #[inline]
    pub fn unregister_gatt_client(&self, client_if: u8) -> Result<(), DispatchError> {
        cmif::unregister_gatt_client(&self.0, client_if)
    }

    /// UnregisterAllGattClients (cmd 64).
    #[inline]
    pub fn unregister_all_gatt_clients(&self) -> Result<(), DispatchError> {
        cmif::unregister_all_gatt_clients(&self.0)
    }

    /// ConnectGattServer (cmd 65).
    #[inline]
    pub fn connect_gatt_server(
        &self,
        client_if: u8,
        addr: BtdrvAddress,
        is_direct: bool,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::connect_gatt_server(
            &self.0,
            ConnectGattServerIn {
                client_if,
                addr,
                is_direct: is_direct as u8,
                applet_resource_user_id,
            },
        )
    }

    /// CancelConnectGattServer (cmd 66, 5.1.0+).
    #[inline]
    pub fn cancel_connect_gatt_server(
        &self,
        client_if: u8,
        addr: BtdrvAddress,
        is_direct: bool,
    ) -> Result<(), DispatchError> {
        cmif::cancel_connect_gatt_server(
            &self.0,
            CancelConnectGattServerIn {
                client_if,
                addr,
                is_direct: is_direct as u8,
            },
        )
    }

    /// DisconnectGattServer (cmd 67, 5.1.0+).
    #[inline]
    pub fn disconnect_gatt_server(&self, conn_id: u32) -> Result<(), DispatchError> {
        cmif::disconnect_gatt_server(&self.0, conn_id, proto::DISCONNECT_GATT_SERVER)
    }

    /// DisconnectGattServer \[pre-5.1.0\] (cmd 66).
    #[inline]
    pub fn disconnect_gatt_server_legacy(&self, conn_id: u32) -> Result<(), DispatchError> {
        cmif::disconnect_gatt_server(&self.0, conn_id, proto::DISCONNECT_GATT_SERVER_LEGACY)
    }

    /// GetGattAttribute \[9.0.0+\] (cmd 68, 5.1.0+).
    #[inline]
    pub fn get_gatt_attribute(&self, conn_id: u32) -> Result<(), DispatchError> {
        cmif::get_gatt_attribute(&self.0, conn_id, proto::GET_GATT_ATTRIBUTE)
    }

    /// GetGattAttribute \[9.0.0+, pre-5.1.0\] (cmd 67).
    #[inline]
    pub fn get_gatt_attribute_legacy(&self, conn_id: u32) -> Result<(), DispatchError> {
        cmif::get_gatt_attribute(&self.0, conn_id, proto::GET_GATT_ATTRIBUTE_LEGACY)
    }

    /// GetGattAttribute \[pre-9.0.0\] (cmd 68, 5.1.0+).
    #[inline]
    pub fn get_gatt_attribute_with_addr(
        &self,
        addr: BtdrvAddress,
        conn_id: u32,
    ) -> Result<(), DispatchError> {
        cmif::get_gatt_attribute_legacy(
            &self.0,
            GetGattAttributeLegacyIn {
                addr,
                pad: [0; 2],
                conn_id,
            },
            proto::GET_GATT_ATTRIBUTE,
        )
    }

    /// GetGattAttribute \[pre-9.0.0, pre-5.1.0\] (cmd 67).
    #[inline]
    pub fn get_gatt_attribute_with_addr_legacy(
        &self,
        addr: BtdrvAddress,
        conn_id: u32,
    ) -> Result<(), DispatchError> {
        cmif::get_gatt_attribute_legacy(
            &self.0,
            GetGattAttributeLegacyIn {
                addr,
                pad: [0; 2],
                conn_id,
            },
            proto::GET_GATT_ATTRIBUTE_LEGACY,
        )
    }

    /// GetGattService (cmd 69, 5.1.0+).
    #[inline]
    pub fn get_gatt_service(
        &self,
        uuid: &BtdrvGattAttributeUuid,
        conn_id: u32,
    ) -> Result<(), DispatchError> {
        cmif::get_gatt_service(
            &self.0,
            GetGattServiceIn {
                conn_id,
                uuid: *uuid,
            },
            proto::GET_GATT_SERVICE,
        )
    }

    /// GetGattService \[pre-5.1.0\] (cmd 68).
    #[inline]
    pub fn get_gatt_service_legacy(
        &self,
        uuid: &BtdrvGattAttributeUuid,
        conn_id: u32,
    ) -> Result<(), DispatchError> {
        cmif::get_gatt_service(
            &self.0,
            GetGattServiceIn {
                conn_id,
                uuid: *uuid,
            },
            proto::GET_GATT_SERVICE_LEGACY,
        )
    }

    /// ConfigureAttMtu (cmd 70, 5.1.0+).
    #[inline]
    pub fn configure_att_mtu(&self, conn_id: u32, mtu: u16) -> Result<(), DispatchError> {
        cmif::configure_att_mtu(
            &self.0,
            ConfigureAttMtuIn {
                mtu,
                pad: 0,
                conn_id,
            },
            proto::CONFIGURE_ATT_MTU,
        )
    }

    /// ConfigureAttMtu \[pre-5.1.0\] (cmd 69).
    #[inline]
    pub fn configure_att_mtu_legacy(&self, conn_id: u32, mtu: u16) -> Result<(), DispatchError> {
        cmif::configure_att_mtu(
            &self.0,
            ConfigureAttMtuIn {
                mtu,
                pad: 0,
                conn_id,
            },
            proto::CONFIGURE_ATT_MTU_LEGACY,
        )
    }

    /// RegisterGattServer (cmd 71, 5.1.0+).
    #[inline]
    pub fn register_gatt_server(&self, uuid: &BtdrvGattAttributeUuid) -> Result<(), DispatchError> {
        cmif::register_gatt_server(&self.0, *uuid, proto::REGISTER_GATT_SERVER)
    }

    /// RegisterGattServer \[pre-5.1.0\] (cmd 70).
    #[inline]
    pub fn register_gatt_server_legacy(
        &self,
        uuid: &BtdrvGattAttributeUuid,
    ) -> Result<(), DispatchError> {
        cmif::register_gatt_server(&self.0, *uuid, proto::REGISTER_GATT_SERVER_LEGACY)
    }

    /// UnregisterGattServer (cmd 72, 5.1.0+).
    #[inline]
    pub fn unregister_gatt_server(&self, server_if: u8) -> Result<(), DispatchError> {
        cmif::unregister_gatt_server(&self.0, server_if, proto::UNREGISTER_GATT_SERVER)
    }

    /// UnregisterGattServer \[pre-5.1.0\] (cmd 71).
    #[inline]
    pub fn unregister_gatt_server_legacy(&self, server_if: u8) -> Result<(), DispatchError> {
        cmif::unregister_gatt_server(&self.0, server_if, proto::UNREGISTER_GATT_SERVER_LEGACY)
    }

    /// ConnectGattClient (cmd 73, 5.1.0+).
    #[inline]
    pub fn connect_gatt_client(
        &self,
        server_if: u8,
        addr: BtdrvAddress,
        is_direct: bool,
    ) -> Result<(), DispatchError> {
        cmif::connect_gatt_client(
            &self.0,
            ConnectGattClientIn {
                server_if,
                addr,
                is_direct: is_direct as u8,
            },
            proto::CONNECT_GATT_CLIENT,
        )
    }

    /// ConnectGattClient \[pre-5.1.0\] (cmd 72).
    #[inline]
    pub fn connect_gatt_client_legacy(
        &self,
        server_if: u8,
        addr: BtdrvAddress,
        is_direct: bool,
    ) -> Result<(), DispatchError> {
        cmif::connect_gatt_client(
            &self.0,
            ConnectGattClientIn {
                server_if,
                addr,
                is_direct: is_direct as u8,
            },
            proto::CONNECT_GATT_CLIENT_LEGACY,
        )
    }

    /// DisconnectGattClient \[9.0.0+\] (cmd 74, 5.1.0+).
    #[inline]
    pub fn disconnect_gatt_client(&self, conn_id: u8) -> Result<(), DispatchError> {
        cmif::disconnect_gatt_client(&self.0, conn_id, proto::DISCONNECT_GATT_CLIENT)
    }

    /// DisconnectGattClient \[9.0.0+, pre-5.1.0\] (cmd 73).
    #[inline]
    pub fn disconnect_gatt_client_legacy(&self, conn_id: u8) -> Result<(), DispatchError> {
        cmif::disconnect_gatt_client(&self.0, conn_id, proto::DISCONNECT_GATT_CLIENT_LEGACY)
    }

    /// DisconnectGattClient \[pre-9.0.0\] (cmd 74, 5.1.0+).
    #[inline]
    pub fn disconnect_gatt_client_with_addr(
        &self,
        conn_id: u8,
        addr: BtdrvAddress,
    ) -> Result<(), DispatchError> {
        cmif::disconnect_gatt_client_legacy(
            &self.0,
            DisconnectGattClientLegacyIn { conn_id, addr },
            proto::DISCONNECT_GATT_CLIENT,
        )
    }

    /// DisconnectGattClient \[pre-9.0.0, pre-5.1.0\] (cmd 73).
    #[inline]
    pub fn disconnect_gatt_client_with_addr_legacy(
        &self,
        conn_id: u8,
        addr: BtdrvAddress,
    ) -> Result<(), DispatchError> {
        cmif::disconnect_gatt_client_legacy(
            &self.0,
            DisconnectGattClientLegacyIn { conn_id, addr },
            proto::DISCONNECT_GATT_CLIENT_LEGACY,
        )
    }

    /// AddGattService (cmd 75).
    #[inline]
    pub fn add_gatt_service(
        &self,
        server_if: u8,
        uuid: &BtdrvGattAttributeUuid,
        num_handle: u8,
        is_primary: bool,
    ) -> Result<(), DispatchError> {
        cmif::add_gatt_service(
            &self.0,
            AddGattServiceIn {
                server_if,
                num_handle,
                is_primary: is_primary as u8,
                pad: 0,
                uuid: *uuid,
            },
        )
    }

    /// EnableGattService (cmd 76, 5.1.0+).
    #[inline]
    pub fn enable_gatt_service(
        &self,
        server_if: u8,
        uuid: &BtdrvGattAttributeUuid,
    ) -> Result<(), DispatchError> {
        cmif::enable_gatt_service(
            &self.0,
            EnableGattServiceIn {
                server_if,
                pad: [0; 3],
                uuid: *uuid,
            },
            proto::ENABLE_GATT_SERVICE,
        )
    }

    /// EnableGattService \[pre-5.1.0\] (cmd 74).
    #[inline]
    pub fn enable_gatt_service_legacy(
        &self,
        server_if: u8,
        uuid: &BtdrvGattAttributeUuid,
    ) -> Result<(), DispatchError> {
        cmif::enable_gatt_service(
            &self.0,
            EnableGattServiceIn {
                server_if,
                pad: [0; 3],
                uuid: *uuid,
            },
            proto::ENABLE_GATT_SERVICE_LEGACY,
        )
    }

    /// AddGattCharacteristic (cmd 77).
    #[inline]
    pub fn add_gatt_characteristic(
        &self,
        server_if: u8,
        serv_uuid: &BtdrvGattAttributeUuid,
        char_uuid: &BtdrvGattAttributeUuid,
        permissions: u16,
        property: u8,
    ) -> Result<(), DispatchError> {
        cmif::add_gatt_characteristic(
            &self.0,
            AddGattCharacteristicIn {
                server_if,
                property,
                permissions,
                serv_uuid: *serv_uuid,
                char_uuid: *char_uuid,
            },
        )
    }

    /// AddGattDescriptor (cmd 78, 5.1.0+).
    #[inline]
    pub fn add_gatt_descriptor(
        &self,
        server_if: u8,
        serv_uuid: &BtdrvGattAttributeUuid,
        desc_uuid: &BtdrvGattAttributeUuid,
        permissions: u16,
    ) -> Result<(), DispatchError> {
        cmif::add_gatt_descriptor(
            &self.0,
            AddGattDescriptorIn {
                server_if,
                pad: 0,
                permissions,
                serv_uuid: *serv_uuid,
                desc_uuid: *desc_uuid,
            },
            proto::ADD_GATT_DESCRIPTOR,
        )
    }

    /// AddGattDescriptor \[pre-5.1.0\] (cmd 76).
    #[inline]
    pub fn add_gatt_descriptor_legacy(
        &self,
        server_if: u8,
        serv_uuid: &BtdrvGattAttributeUuid,
        desc_uuid: &BtdrvGattAttributeUuid,
        permissions: u16,
    ) -> Result<(), DispatchError> {
        cmif::add_gatt_descriptor(
            &self.0,
            AddGattDescriptorIn {
                server_if,
                pad: 0,
                permissions,
                serv_uuid: *serv_uuid,
                desc_uuid: *desc_uuid,
            },
            proto::ADD_GATT_DESCRIPTOR_LEGACY,
        )
    }

    /// GetBleManagedEventInfo (cmd 79, 5.1.0+). Returns event type.
    #[inline]
    pub fn get_ble_managed_event_info(&self, buf: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::get_ble_managed_event_info(&self.0, buf, proto::GET_BLE_MANAGED_EVENT_INFO)
    }

    /// GetBleManagedEventInfo \[pre-5.1.0\] (cmd 78). Returns event type.
    #[inline]
    pub fn get_ble_managed_event_info_legacy(&self, buf: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::get_ble_managed_event_info(&self.0, buf, proto::GET_BLE_MANAGED_EVENT_INFO_LEGACY)
    }

    /// GetGattFirstCharacteristic (cmd 80, 5.1.0+).
    /// Returns `(property, gatt_id)`.
    #[inline]
    pub fn get_gatt_first_characteristic(
        &self,
        conn_id: u32,
        serv_id: &BtdrvGattId,
        is_primary: bool,
        filter_uuid: &BtdrvGattAttributeUuid,
    ) -> Result<(u8, BtdrvGattId), DispatchError> {
        let out = cmif::get_gatt_first_characteristic(
            &self.0,
            GetGattFirstCharacteristicIn {
                is_primary: is_primary as u8,
                pad: [0; 3],
                conn_id,
                serv_id: *serv_id,
                filter_uuid: *filter_uuid,
            },
            proto::GET_GATT_FIRST_CHARACTERISTIC,
        )?;
        Ok((out.property, out.id))
    }

    /// GetGattFirstCharacteristic \[pre-5.1.0\] (cmd 79).
    /// Returns `(property, gatt_id)`.
    #[inline]
    pub fn get_gatt_first_characteristic_legacy(
        &self,
        conn_id: u32,
        serv_id: &BtdrvGattId,
        is_primary: bool,
        filter_uuid: &BtdrvGattAttributeUuid,
    ) -> Result<(u8, BtdrvGattId), DispatchError> {
        let out = cmif::get_gatt_first_characteristic(
            &self.0,
            GetGattFirstCharacteristicIn {
                is_primary: is_primary as u8,
                pad: [0; 3],
                conn_id,
                serv_id: *serv_id,
                filter_uuid: *filter_uuid,
            },
            proto::GET_GATT_FIRST_CHARACTERISTIC_LEGACY,
        )?;
        Ok((out.property, out.id))
    }

    /// GetGattNextCharacteristic (cmd 81, 5.1.0+).
    /// Returns `(property, gatt_id)`.
    #[inline]
    pub fn get_gatt_next_characteristic(
        &self,
        conn_id: u32,
        serv_id: &BtdrvGattId,
        is_primary: bool,
        char_id: &BtdrvGattId,
        filter_uuid: &BtdrvGattAttributeUuid,
    ) -> Result<(u8, BtdrvGattId), DispatchError> {
        let out = cmif::get_gatt_next_characteristic(
            &self.0,
            GetGattNextCharacteristicIn {
                is_primary: is_primary as u8,
                pad: [0; 3],
                conn_id,
                serv_id: *serv_id,
                char_id: *char_id,
                filter_uuid: *filter_uuid,
            },
            proto::GET_GATT_NEXT_CHARACTERISTIC,
        )?;
        Ok((out.property, out.id))
    }

    /// GetGattNextCharacteristic \[pre-5.1.0\] (cmd 80).
    /// Returns `(property, gatt_id)`.
    #[inline]
    pub fn get_gatt_next_characteristic_legacy(
        &self,
        conn_id: u32,
        serv_id: &BtdrvGattId,
        is_primary: bool,
        char_id: &BtdrvGattId,
        filter_uuid: &BtdrvGattAttributeUuid,
    ) -> Result<(u8, BtdrvGattId), DispatchError> {
        let out = cmif::get_gatt_next_characteristic(
            &self.0,
            GetGattNextCharacteristicIn {
                is_primary: is_primary as u8,
                pad: [0; 3],
                conn_id,
                serv_id: *serv_id,
                char_id: *char_id,
                filter_uuid: *filter_uuid,
            },
            proto::GET_GATT_NEXT_CHARACTERISTIC_LEGACY,
        )?;
        Ok((out.property, out.id))
    }

    /// GetGattFirstDescriptor (cmd 82, 5.1.0+).
    #[inline]
    pub fn get_gatt_first_descriptor(
        &self,
        conn_id: u32,
        serv_id: &BtdrvGattId,
        is_primary: bool,
        char_id: &BtdrvGattId,
        filter_uuid: &BtdrvGattAttributeUuid,
    ) -> Result<BtdrvGattId, DispatchError> {
        cmif::get_gatt_first_descriptor(
            &self.0,
            GetGattFirstDescriptorIn {
                is_primary: is_primary as u8,
                pad: [0; 3],
                conn_id,
                serv_id: *serv_id,
                char_id: *char_id,
                filter_uuid: *filter_uuid,
            },
            proto::GET_GATT_FIRST_DESCRIPTOR,
        )
    }

    /// GetGattFirstDescriptor \[pre-5.1.0\] (cmd 81).
    #[inline]
    pub fn get_gatt_first_descriptor_legacy(
        &self,
        conn_id: u32,
        serv_id: &BtdrvGattId,
        is_primary: bool,
        char_id: &BtdrvGattId,
        filter_uuid: &BtdrvGattAttributeUuid,
    ) -> Result<BtdrvGattId, DispatchError> {
        cmif::get_gatt_first_descriptor(
            &self.0,
            GetGattFirstDescriptorIn {
                is_primary: is_primary as u8,
                pad: [0; 3],
                conn_id,
                serv_id: *serv_id,
                char_id: *char_id,
                filter_uuid: *filter_uuid,
            },
            proto::GET_GATT_FIRST_DESCRIPTOR_LEGACY,
        )
    }

    /// GetGattNextDescriptor (cmd 83, 5.1.0+).
    #[inline]
    pub fn get_gatt_next_descriptor(
        &self,
        conn_id: u32,
        serv_id: &BtdrvGattId,
        is_primary: bool,
        char_id: &BtdrvGattId,
        desc_id: &BtdrvGattId,
        filter_uuid: &BtdrvGattAttributeUuid,
    ) -> Result<BtdrvGattId, DispatchError> {
        cmif::get_gatt_next_descriptor(
            &self.0,
            GetGattNextDescriptorIn {
                is_primary: is_primary as u8,
                pad: [0; 3],
                conn_id,
                serv_id: *serv_id,
                char_id: *char_id,
                desc_id: *desc_id,
                filter_uuid: *filter_uuid,
            },
            proto::GET_GATT_NEXT_DESCRIPTOR,
        )
    }

    /// GetGattNextDescriptor \[pre-5.1.0\] (cmd 82).
    #[inline]
    pub fn get_gatt_next_descriptor_legacy(
        &self,
        conn_id: u32,
        serv_id: &BtdrvGattId,
        is_primary: bool,
        char_id: &BtdrvGattId,
        desc_id: &BtdrvGattId,
        filter_uuid: &BtdrvGattAttributeUuid,
    ) -> Result<BtdrvGattId, DispatchError> {
        cmif::get_gatt_next_descriptor(
            &self.0,
            GetGattNextDescriptorIn {
                is_primary: is_primary as u8,
                pad: [0; 3],
                conn_id,
                serv_id: *serv_id,
                char_id: *char_id,
                desc_id: *desc_id,
                filter_uuid: *filter_uuid,
            },
            proto::GET_GATT_NEXT_DESCRIPTOR_LEGACY,
        )
    }

    /// RegisterGattManagedDataPath (cmd 84).
    #[inline]
    pub fn register_gatt_managed_data_path(
        &self,
        uuid: &BtdrvGattAttributeUuid,
    ) -> Result<(), DispatchError> {
        cmif::register_gatt_managed_data_path(&self.0, *uuid)
    }

    /// UnregisterGattManagedDataPath (cmd 85).
    #[inline]
    pub fn unregister_gatt_managed_data_path(
        &self,
        uuid: &BtdrvGattAttributeUuid,
    ) -> Result<(), DispatchError> {
        cmif::unregister_gatt_managed_data_path(&self.0, *uuid)
    }

    /// RegisterGattHidDataPath (cmd 86).
    #[inline]
    pub fn register_gatt_hid_data_path(
        &self,
        uuid: &BtdrvGattAttributeUuid,
    ) -> Result<(), DispatchError> {
        cmif::register_gatt_hid_data_path(&self.0, *uuid)
    }

    /// UnregisterGattHidDataPath (cmd 87).
    #[inline]
    pub fn unregister_gatt_hid_data_path(
        &self,
        uuid: &BtdrvGattAttributeUuid,
    ) -> Result<(), DispatchError> {
        cmif::unregister_gatt_hid_data_path(&self.0, *uuid)
    }

    /// RegisterGattDataPath (cmd 88).
    #[inline]
    pub fn register_gatt_data_path(
        &self,
        uuid: &BtdrvGattAttributeUuid,
    ) -> Result<(), DispatchError> {
        cmif::register_gatt_data_path(&self.0, *uuid)
    }

    /// UnregisterGattDataPath (cmd 89, 5.1.0+).
    #[inline]
    pub fn unregister_gatt_data_path(
        &self,
        uuid: &BtdrvGattAttributeUuid,
    ) -> Result<(), DispatchError> {
        cmif::unregister_gatt_data_path(&self.0, *uuid, proto::UNREGISTER_GATT_DATA_PATH)
    }

    /// UnregisterGattDataPath \[pre-5.1.0\] (cmd 83).
    #[inline]
    pub fn unregister_gatt_data_path_legacy(
        &self,
        uuid: &BtdrvGattAttributeUuid,
    ) -> Result<(), DispatchError> {
        cmif::unregister_gatt_data_path(&self.0, *uuid, proto::UNREGISTER_GATT_DATA_PATH_LEGACY)
    }

    /// ReadGattCharacteristic (cmd 90, 5.1.0+).
    #[inline]
    pub fn read_gatt_characteristic(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
        auth_req: u8,
    ) -> Result<(), DispatchError> {
        cmif::read_gatt_characteristic(
            &self.0,
            ReadGattCharacteristicIn {
                is_primary: is_primary as u8,
                auth_req,
                pad: [0; 2],
                connection_handle,
                serv_id: *serv_id,
                char_id: *char_id,
            },
            proto::READ_GATT_CHARACTERISTIC,
        )
    }

    /// ReadGattCharacteristic \[pre-5.1.0\] (cmd 89).
    #[inline]
    pub fn read_gatt_characteristic_legacy(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
        auth_req: u8,
    ) -> Result<(), DispatchError> {
        cmif::read_gatt_characteristic(
            &self.0,
            ReadGattCharacteristicIn {
                is_primary: is_primary as u8,
                auth_req,
                pad: [0; 2],
                connection_handle,
                serv_id: *serv_id,
                char_id: *char_id,
            },
            proto::READ_GATT_CHARACTERISTIC_LEGACY,
        )
    }

    /// ReadGattDescriptor (cmd 91, 5.1.0+).
    #[inline]
    pub fn read_gatt_descriptor(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
        desc_id: &BtdrvGattId,
        auth_req: u8,
    ) -> Result<(), DispatchError> {
        cmif::read_gatt_descriptor(
            &self.0,
            ReadGattDescriptorIn {
                is_primary: is_primary as u8,
                auth_req,
                pad: [0; 2],
                connection_handle,
                serv_id: *serv_id,
                char_id: *char_id,
                desc_id: *desc_id,
            },
            proto::READ_GATT_DESCRIPTOR,
        )
    }

    /// ReadGattDescriptor \[pre-5.1.0\] (cmd 90).
    #[inline]
    pub fn read_gatt_descriptor_legacy(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
        desc_id: &BtdrvGattId,
        auth_req: u8,
    ) -> Result<(), DispatchError> {
        cmif::read_gatt_descriptor(
            &self.0,
            ReadGattDescriptorIn {
                is_primary: is_primary as u8,
                auth_req,
                pad: [0; 2],
                connection_handle,
                serv_id: *serv_id,
                char_id: *char_id,
                desc_id: *desc_id,
            },
            proto::READ_GATT_DESCRIPTOR_LEGACY,
        )
    }

    /// WriteGattCharacteristic (cmd 92, 5.1.0+).
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn write_gatt_characteristic(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
        buf: &[u8],
        auth_req: u8,
        with_response: bool,
    ) -> Result<(), DispatchError> {
        cmif::write_gatt_characteristic(
            &self.0,
            WriteGattCharacteristicIn {
                is_primary: is_primary as u8,
                auth_req,
                with_response: with_response as u8,
                pad: 0,
                connection_handle,
                serv_id: *serv_id,
                char_id: *char_id,
            },
            buf,
            proto::WRITE_GATT_CHARACTERISTIC,
        )
    }

    /// WriteGattCharacteristic \[pre-5.1.0\] (cmd 91).
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn write_gatt_characteristic_legacy(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
        buf: &[u8],
        auth_req: u8,
        with_response: bool,
    ) -> Result<(), DispatchError> {
        cmif::write_gatt_characteristic(
            &self.0,
            WriteGattCharacteristicIn {
                is_primary: is_primary as u8,
                auth_req,
                with_response: with_response as u8,
                pad: 0,
                connection_handle,
                serv_id: *serv_id,
                char_id: *char_id,
            },
            buf,
            proto::WRITE_GATT_CHARACTERISTIC_LEGACY,
        )
    }

    /// WriteGattDescriptor (cmd 93, 5.1.0+).
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn write_gatt_descriptor(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
        desc_id: &BtdrvGattId,
        buf: &[u8],
        auth_req: u8,
    ) -> Result<(), DispatchError> {
        cmif::write_gatt_descriptor(
            &self.0,
            WriteGattDescriptorIn {
                is_primary: is_primary as u8,
                auth_req,
                pad: [0; 2],
                connection_handle,
                serv_id: *serv_id,
                char_id: *char_id,
                desc_id: *desc_id,
            },
            buf,
            proto::WRITE_GATT_DESCRIPTOR,
        )
    }

    /// WriteGattDescriptor \[pre-5.1.0\] (cmd 92).
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn write_gatt_descriptor_legacy(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
        desc_id: &BtdrvGattId,
        buf: &[u8],
        auth_req: u8,
    ) -> Result<(), DispatchError> {
        cmif::write_gatt_descriptor(
            &self.0,
            WriteGattDescriptorIn {
                is_primary: is_primary as u8,
                auth_req,
                pad: [0; 2],
                connection_handle,
                serv_id: *serv_id,
                char_id: *char_id,
                desc_id: *desc_id,
            },
            buf,
            proto::WRITE_GATT_DESCRIPTOR_LEGACY,
        )
    }

    /// RegisterGattNotification (cmd 94).
    #[inline]
    pub fn register_gatt_notification(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
    ) -> Result<(), DispatchError> {
        cmif::register_gatt_notification(
            &self.0,
            GattNotificationIn {
                is_primary: is_primary as u8,
                pad: [0; 3],
                connection_handle,
                serv_id: *serv_id,
                char_id: *char_id,
            },
        )
    }

    /// UnregisterGattNotification (cmd 95, 5.1.0+).
    #[inline]
    pub fn unregister_gatt_notification(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
    ) -> Result<(), DispatchError> {
        cmif::unregister_gatt_notification(
            &self.0,
            GattNotificationIn {
                is_primary: is_primary as u8,
                pad: [0; 3],
                connection_handle,
                serv_id: *serv_id,
                char_id: *char_id,
            },
            proto::UNREGISTER_GATT_NOTIFICATION,
        )
    }

    /// UnregisterGattNotification \[pre-5.1.0\] (cmd 93).
    #[inline]
    pub fn unregister_gatt_notification_legacy(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
    ) -> Result<(), DispatchError> {
        cmif::unregister_gatt_notification(
            &self.0,
            GattNotificationIn {
                is_primary: is_primary as u8,
                pad: [0; 3],
                connection_handle,
                serv_id: *serv_id,
                char_id: *char_id,
            },
            proto::UNREGISTER_GATT_NOTIFICATION_LEGACY,
        )
    }

    /// GetLeHidEventInfo (cmd 96, 5.1.0+). Returns event type.
    #[inline]
    pub fn get_le_hid_event_info(&self, buf: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::get_le_hid_event_info(&self.0, buf, proto::GET_LE_HID_EVENT_INFO)
    }

    /// GetLeHidEventInfo \[pre-5.1.0\] (cmd 95). Returns event type.
    #[inline]
    pub fn get_le_hid_event_info_legacy(&self, buf: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::get_le_hid_event_info(&self.0, buf, proto::GET_LE_HID_EVENT_INFO_LEGACY)
    }

    /// RegisterBleHidEvent (cmd 97, 5.1.0+). Returns event handle.
    #[inline]
    pub fn register_ble_hid_event(&self) -> Result<u32, AcquireEventError> {
        cmif::register_ble_hid_event(&self.0, proto::REGISTER_BLE_HID_EVENT)
    }

    /// RegisterBleHidEvent \[pre-5.1.0\] (cmd 96). Returns event handle.
    #[inline]
    pub fn register_ble_hid_event_legacy(&self) -> Result<u32, AcquireEventError> {
        cmif::register_ble_hid_event(&self.0, proto::REGISTER_BLE_HID_EVENT_LEGACY)
    }

    /// SetBleScanParameter (cmd 98).
    #[inline]
    pub fn set_ble_scan_parameter(
        &self,
        scan_interval: u16,
        scan_window: u16,
    ) -> Result<(), DispatchError> {
        cmif::set_ble_scan_parameter(
            &self.0,
            SetBleScanParameterIn {
                scan_interval,
                scan_window,
            },
        )
    }
}

// Other

impl BtdrvService {
    /// MoveToSecondaryPiconet (cmd 99, 10.0.0+).
    #[inline]
    pub fn move_to_secondary_piconet(&self, addr: BtdrvAddress) -> Result<(), DispatchError> {
        cmif::move_to_secondary_piconet(&self.0, addr)
    }

    /// IsBluetoothEnabled (cmd 100, 12.0.0+).
    #[inline]
    pub fn is_bluetooth_enabled(&self) -> Result<u8, DispatchError> {
        cmif::is_bluetooth_enabled(&self.0)
    }
}

// Audio

impl BtdrvService {
    /// AcquireAudioEvent (cmd 128). Returns event handle.
    #[inline]
    pub fn acquire_audio_event(&self) -> Result<u32, AcquireEventError> {
        cmif::acquire_audio_event(&self.0)
    }

    /// GetAudioEventInfo (cmd 129). Returns event type.
    #[inline]
    pub fn get_audio_event_info(&self, buf: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::get_audio_event_info(&self.0, buf)
    }

    /// OpenAudioConnection (cmd 130).
    #[inline]
    pub fn open_audio_connection(&self, addr: BtdrvAddress) -> Result<(), DispatchError> {
        cmif::open_audio_connection(&self.0, addr)
    }

    /// CloseAudioConnection (cmd 131).
    #[inline]
    pub fn close_audio_connection(&self, addr: BtdrvAddress) -> Result<(), DispatchError> {
        cmif::close_audio_connection(&self.0, addr)
    }

    /// OpenAudioOut (cmd 132). Returns audio handle.
    #[inline]
    pub fn open_audio_out(&self, addr: BtdrvAddress) -> Result<u32, DispatchError> {
        cmif::open_audio_out(&self.0, addr)
    }

    /// CloseAudioOut (cmd 133).
    #[inline]
    pub fn close_audio_out(&self, audio_handle: u32) -> Result<(), DispatchError> {
        cmif::close_audio_out(&self.0, audio_handle)
    }

    /// AcquireAudioOutStateChangedEvent (cmd 134). Returns event handle.
    #[inline]
    pub fn acquire_audio_out_state_changed_event(
        &self,
        audio_handle: u32,
    ) -> Result<u32, AcquireEventError> {
        cmif::acquire_audio_out_state_changed_event(&self.0, audio_handle)
    }

    /// StartAudioOut (cmd 135). Returns `(latency, out1)`.
    #[inline]
    pub fn start_audio_out(
        &self,
        audio_handle: u32,
        pcm_param: &BtdrvPcmParameter,
        latency: i64,
    ) -> Result<(i64, u64), DispatchError> {
        let out = cmif::start_audio_out(
            &self.0,
            StartAudioOutIn {
                audio_handle,
                pcm_param: *pcm_param,
                latency,
            },
        )?;
        Ok((out.latency, out.out1))
    }

    /// StopAudioOut (cmd 136).
    #[inline]
    pub fn stop_audio_out(&self, audio_handle: u32) -> Result<(), DispatchError> {
        cmif::stop_audio_out(&self.0, audio_handle)
    }

    /// GetAudioOutState (cmd 137).
    #[inline]
    pub fn get_audio_out_state(&self, audio_handle: u32) -> Result<u32, DispatchError> {
        cmif::get_audio_out_state(&self.0, audio_handle)
    }

    /// GetAudioOutFeedingCodec (cmd 138).
    #[inline]
    pub fn get_audio_out_feeding_codec(&self, audio_handle: u32) -> Result<u32, DispatchError> {
        cmif::get_audio_out_feeding_codec(&self.0, audio_handle)
    }

    /// GetAudioOutFeedingParameter (cmd 139).
    #[inline]
    pub fn get_audio_out_feeding_parameter(
        &self,
        audio_handle: u32,
    ) -> Result<BtdrvPcmParameter, DispatchError> {
        cmif::get_audio_out_feeding_parameter(&self.0, audio_handle)
    }

    /// AcquireAudioOutBufferAvailableEvent (cmd 140). Returns event handle.
    #[inline]
    pub fn acquire_audio_out_buffer_available_event(
        &self,
        audio_handle: u32,
    ) -> Result<u32, AcquireEventError> {
        cmif::acquire_audio_out_buffer_available_event(&self.0, audio_handle)
    }

    /// SendAudioData (cmd 141). Returns bytes consumed.
    #[inline]
    pub fn send_audio_data(&self, audio_handle: u32, buf: &[u8]) -> Result<u64, DispatchError> {
        cmif::send_audio_data(&self.0, audio_handle, buf)
    }

    /// AcquireAudioControlInputStateChangedEvent (cmd 142). Returns event handle.
    #[inline]
    pub fn acquire_audio_control_input_state_changed_event(
        &self,
    ) -> Result<u32, AcquireEventError> {
        cmif::acquire_audio_control_input_state_changed_event(&self.0)
    }

    /// GetAudioControlInputState (cmd 143). Returns count.
    #[inline]
    pub fn get_audio_control_input_state(&self, buf: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::get_audio_control_input_state(&self.0, buf)
    }

    /// AcquireAudioConnectionStateChangedEvent (cmd 144). Returns event handle.
    #[inline]
    pub fn acquire_audio_connection_state_changed_event(&self) -> Result<u32, AcquireEventError> {
        cmif::acquire_audio_connection_state_changed_event(&self.0)
    }

    /// GetConnectedAudioDevice (cmd 145). Returns count.
    #[inline]
    pub fn get_connected_audio_device(&self, buf: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::get_connected_audio_device(&self.0, buf)
    }

    /// CloseAudioControlInput (cmd 146).
    #[inline]
    pub fn close_audio_control_input(&self, addr: BtdrvAddress) -> Result<(), DispatchError> {
        cmif::close_audio_control_input(&self.0, addr)
    }

    /// RegisterAudioControlNotification (cmd 147).
    #[inline]
    pub fn register_audio_control_notification(
        &self,
        addr: BtdrvAddress,
        event_type: u32,
    ) -> Result<(), DispatchError> {
        cmif::register_audio_control_notification(
            &self.0,
            AddrU32In {
                addr,
                pad: [0; 2],
                val: event_type,
            },
        )
    }

    /// SendAudioControlPassthroughCommand (cmd 148).
    #[inline]
    pub fn send_audio_control_passthrough_command(
        &self,
        addr: BtdrvAddress,
        op_id: u32,
        state_flag: u32,
    ) -> Result<(), DispatchError> {
        cmif::send_audio_control_passthrough_command(
            &self.0,
            AddrU32U32In {
                addr,
                pad: [0; 2],
                val0: op_id,
                val1: state_flag,
            },
        )
    }

    /// SendAudioControlSetAbsoluteVolumeCommand (cmd 149).
    #[inline]
    pub fn send_audio_control_set_absolute_volume_command(
        &self,
        addr: BtdrvAddress,
        volume: i32,
    ) -> Result<(), DispatchError> {
        cmif::send_audio_control_set_absolute_volume_command(
            &self.0,
            AddrU32In {
                addr,
                pad: [0; 2],
                val: volume as u32,
            },
        )
    }
}

// Debug

impl BtdrvService {
    /// IsManufacturingMode (cmd 256).
    #[inline]
    pub fn is_manufacturing_mode(&self) -> Result<u8, DispatchError> {
        cmif::is_manufacturing_mode(&self.0)
    }

    /// EmulateBluetoothCrash (cmd 257).
    #[inline]
    pub fn emulate_bluetooth_crash(&self, reason: BtdrvFatalReason) -> Result<(), DispatchError> {
        cmif::emulate_bluetooth_crash(&self.0, reason as u32)
    }

    /// GetBleChannelMap (cmd 258).
    #[inline]
    pub fn get_ble_channel_map(&self, out: &mut BtdrvChannelMapList) -> Result<(), DispatchError> {
        cmif::get_ble_channel_map(&self.0, out)
    }
}

// Connection

/// Connects to the `btdrv` service via SM.
///
/// Calls InitializeBluetoothDriver (cmd 0) after obtaining the session.
pub fn connect_cmif(sm: &SmService) -> Result<BtdrvService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    let session = Session::from_handle(handle, 0);
    let service = BtdrvService(session);

    cmif::initialize_bluetooth_driver(&service.0).map_err(ConnectCmifError::Initialize)?;

    Ok(service)
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    #[error("failed to get btdrv service")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    #[error("InitializeBluetoothDriver failed")]
    Initialize(#[source] DispatchError),
}
