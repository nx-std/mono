//! CMIF protocol operations for the Bluetooth Driver service.

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
        dispatch_in_out,
        dispatch_no_io,
        dispatch_out,
    },
    proto,
    types::{
        AddGattCharacteristicIn,
        AddGattDescriptorIn,
        AddGattServiceIn,
        AddrU8In,
        AddrU32In,
        AddrU32U32In,
        BtdrvAdapterProperty,
        BtdrvAdapterPropertyOld,
        BtdrvAdapterPropertySet,
        BtdrvAddress,
        BtdrvBleAdvertiseFilter,
        BtdrvBleAdvertisePacketData,
        BtdrvBleConnectionParameter,
        BtdrvChannelMapList,
        BtdrvGattAttributeUuid,
        BtdrvGattId,
        BtdrvHidData,
        BtdrvHidReport,
        BtdrvLeConnectionParams,
        BtdrvPcmParameter,
        BtdrvPlrList,
        BtdrvPlrStatistics,
        CancelConnectGattServerIn,
        ConfigureAttMtuIn,
        ConnectGattClientIn,
        ConnectGattServerIn,
        DisconnectGattClientLegacyIn,
        EnableGattServiceIn,
        GattNotificationIn,
        GetGattAttributeLegacyIn,
        GetGattCharacteristicOut,
        GetGattFirstCharacteristicIn,
        GetGattFirstDescriptorIn,
        GetGattNextCharacteristicIn,
        GetGattNextDescriptorIn,
        GetGattServiceIn,
        GetHidReportIn,
        LegacyRespondToPinRequestIn,
        ReadGattCharacteristicIn,
        ReadGattDescriptorIn,
        RespondToPinRequestIn,
        RespondToSspRequestIn,
        RespondToSspRequestLegacyIn,
        SetBleAdvertiseParameterIn,
        SetBleConnectionParameterIn,
        SetBleScanParameterIn,
        SetHidReportIn,
        SetSysBluetoothDevicesSettings,
        StartAudioOutIn,
        StartAudioOutOut,
        StartInquiryIn,
        TriggerConnectionIn,
        TwoBoolsIn,
        WriteGattCharacteristicIn,
        WriteGattDescriptorIn,
    },
};

// Core Bluetooth (cmds 0-15)

/// InitializeBluetoothDriver (cmd 0).
pub(crate) fn initialize_bluetooth_driver(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::INITIALIZE_BLUETOOTH_DRIVER)
}

/// InitializeBluetooth (cmd 1) — returns event handle.
pub(crate) fn initialize_bluetooth(service: &Session) -> Result<u32, AcquireEventError> {
    acquire_event(service, proto::INITIALIZE_BLUETOOTH)
}

/// EnableBluetooth (cmd 2).
pub(crate) fn enable_bluetooth(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::ENABLE_BLUETOOTH)
}

/// DisableBluetooth (cmd 3).
pub(crate) fn disable_bluetooth(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::DISABLE_BLUETOOTH)
}

/// FinalizeBluetooth (cmd 4).
pub(crate) fn finalize_bluetooth(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::FINALIZE_BLUETOOTH)
}

/// GetAdapterProperties \[1.0.0-11.0.1\] (cmd 5, HipcPointer fixed out).
pub(crate) fn get_adapter_properties_legacy(
    service: &Session,
    out: &mut BtdrvAdapterPropertyOld,
) -> Result<(), DispatchError> {
    dispatch_out_buf_ptr_fixed(service, out, proto::GET_ADAPTER_PROPERTIES)
}

/// GetAdapterProperties \[12.0.0+\] (cmd 5, HipcPointer fixed out).
pub(crate) fn get_adapter_properties(
    service: &Session,
    out: &mut BtdrvAdapterPropertySet,
) -> Result<(), DispatchError> {
    dispatch_out_buf_ptr_fixed(service, out, proto::GET_ADAPTER_PROPERTIES)
}

/// GetAdapterProperty \[1.0.0-11.0.1\] (cmd 6, in u32 + HipcPointer out variable).
pub(crate) fn get_adapter_property_legacy(
    service: &Session,
    property_type: u32,
    buf: &mut [u8],
) -> Result<(), DispatchError> {
    // SAFETY: `property_type` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const property_type).cast::<u8>(), size_of::<u32>())
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::GET_ADAPTER_PROPERTY)
        .in_raw(in_bytes)
        .out_buffer(buf, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// GetAdapterProperty \[12.0.0+\] (cmd 6, in u32 + HipcPointer fixed out).
pub(crate) fn get_adapter_property(
    service: &Session,
    property_type: u32,
    out: &mut BtdrvAdapterProperty,
) -> Result<(), DispatchError> {
    // SAFETY: `property_type` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const property_type).cast::<u8>(), size_of::<u32>())
    };
    // SAFETY: `out` is a valid mutable reference; viewing its bytes as a
    // slice for the OUT buffer is sound, and the byte slice borrows `out`.
    let out_buf = unsafe {
        core::slice::from_raw_parts_mut(
            (out as *mut BtdrvAdapterProperty).cast::<u8>(),
            size_of::<BtdrvAdapterProperty>(),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::GET_ADAPTER_PROPERTY)
        .in_raw(in_bytes)
        .out_buffer(out_buf, BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE))
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// SetAdapterProperty \[1.0.0-11.0.1\] (cmd 7, in u32 + HipcPointer in variable).
pub(crate) fn set_adapter_property_legacy(
    service: &Session,
    property_type: u32,
    buf: &[u8],
) -> Result<(), DispatchError> {
    // SAFETY: `property_type` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const property_type).cast::<u8>(), size_of::<u32>())
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::SET_ADAPTER_PROPERTY)
        .in_raw(in_bytes)
        .in_buffer(buf, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// SetAdapterProperty \[12.0.0+\] (cmd 7, in u32 + HipcPointer fixed in).
pub(crate) fn set_adapter_property(
    service: &Session,
    property_type: u32,
    input: &BtdrvAdapterProperty,
) -> Result<(), DispatchError> {
    // SAFETY: `property_type` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const property_type).cast::<u8>(), size_of::<u32>())
    };
    // SAFETY: `input` is a valid reference; viewing its bytes as a slice for
    // the IN buffer is sound, and the byte slice borrows `input`.
    let buf = unsafe {
        core::slice::from_raw_parts(
            (input as *const BtdrvAdapterProperty).cast::<u8>(),
            size_of::<BtdrvAdapterProperty>(),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::SET_ADAPTER_PROPERTY)
        .in_raw(in_bytes)
        .in_buffer(buf, BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE))
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// StartInquiry \[1.0.0-11.0.1\] (cmd 8).
pub(crate) fn start_inquiry_legacy(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::START_INQUIRY)
}

/// StartInquiry \[12.0.0+\] (cmd 8).
pub(crate) fn start_inquiry(service: &Session, input: StartInquiryIn) -> Result<(), DispatchError> {
    dispatch_in(service, proto::START_INQUIRY, input)
}

/// StopInquiry (cmd 9).
pub(crate) fn stop_inquiry(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::STOP_INQUIRY)
}

/// CreateBond \[1.0.0-8.1.1\] (cmd 10, in addr + HipcPointer fixed in u32).
pub(crate) fn create_bond_legacy(
    service: &Session,
    addr: BtdrvAddress,
    bond_type: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `addr` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const addr).cast::<u8>(), size_of::<BtdrvAddress>())
    };
    // SAFETY: `bond_type` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let type_buf = unsafe {
        core::slice::from_raw_parts((&raw const bond_type).cast::<u8>(), size_of::<u32>())
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::CREATE_BOND)
        .in_raw(in_bytes)
        .in_buffer(
            type_buf,
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// CreateBond \[9.0.0+\] (cmd 10).
pub(crate) fn create_bond(service: &Session, input: AddrU32In) -> Result<(), DispatchError> {
    dispatch_in(service, proto::CREATE_BOND, input)
}

/// RemoveBond (cmd 11).
pub(crate) fn remove_bond(service: &Session, addr: BtdrvAddress) -> Result<(), DispatchError> {
    dispatch_in(service, proto::REMOVE_BOND, addr)
}

/// CancelBond (cmd 12).
pub(crate) fn cancel_bond(service: &Session, addr: BtdrvAddress) -> Result<(), DispatchError> {
    dispatch_in(service, proto::CANCEL_BOND, addr)
}

/// RespondToPinRequest \[1.0.0-11.0.1\] (cmd 13).
pub(crate) fn respond_to_pin_request_legacy(
    service: &Session,
    input: LegacyRespondToPinRequestIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::RESPOND_TO_PIN_REQUEST, input)
}

/// RespondToPinRequest \[12.0.0+\] (cmd 13).
pub(crate) fn respond_to_pin_request(
    service: &Session,
    input: RespondToPinRequestIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::RESPOND_TO_PIN_REQUEST, input)
}

/// RespondToSspRequest \[1.0.0-11.0.1\] (cmd 14).
pub(crate) fn respond_to_ssp_request_legacy(
    service: &Session,
    input: RespondToSspRequestLegacyIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::RESPOND_TO_SSP_REQUEST, input)
}

/// RespondToSspRequest \[12.0.0+\] (cmd 14).
pub(crate) fn respond_to_ssp_request(
    service: &Session,
    input: RespondToSspRequestIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::RESPOND_TO_SSP_REQUEST, input)
}

/// GetEventInfo (cmd 15, out u32 + HipcPointer out variable).
pub(crate) fn get_event_info(service: &Session, buf: &mut [u8]) -> Result<u32, DispatchError> {
    dispatch_out_u32_out_buf(service, buf, proto::GET_EVENT_INFO)
}

// HID (cmds 16-27)

/// InitializeHid (cmd 16, in u16 + copy handle out).
pub(crate) fn initialize_hid(service: &Session) -> Result<u32, AcquireEventError> {
    dispatch_in_u16_get_handle(service, proto::INITIALIZE_HID, 0x1)
}

/// OpenHidConnection (cmd 17).
pub(crate) fn open_hid_connection(
    service: &Session,
    addr: BtdrvAddress,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::OPEN_HID_CONNECTION, addr)
}

/// CloseHidConnection (cmd 18).
pub(crate) fn close_hid_connection(
    service: &Session,
    addr: BtdrvAddress,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::CLOSE_HID_CONNECTION, addr)
}

/// WriteHidData \[1.0.0-8.1.1\] (cmd 19, in addr + HipcPointer fixed in BtdrvHidData).
pub(crate) fn write_hid_data_legacy(
    service: &Session,
    addr: BtdrvAddress,
    report: &BtdrvHidData,
) -> Result<(), DispatchError> {
    // SAFETY: `addr` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const addr).cast::<u8>(), size_of::<BtdrvAddress>())
    };
    // SAFETY: `report` is a valid reference; viewing its bytes as a slice for
    // the IN buffer is sound, and the byte slice borrows `report`.
    let buf = unsafe {
        core::slice::from_raw_parts(
            (report as *const BtdrvHidData).cast::<u8>(),
            size_of::<BtdrvHidData>(),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::WRITE_HID_DATA)
        .in_raw(in_bytes)
        .in_buffer(buf, BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE))
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// WriteHidData \[9.0.0+\] (cmd 19, in addr + HipcPointer fixed in BtdrvHidReport).
pub(crate) fn write_hid_data(
    service: &Session,
    addr: BtdrvAddress,
    report: &BtdrvHidReport,
) -> Result<(), DispatchError> {
    // SAFETY: `addr` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const addr).cast::<u8>(), size_of::<BtdrvAddress>())
    };
    // SAFETY: `report` is a valid reference; viewing its bytes as a slice for
    // the IN buffer is sound, and the byte slice borrows `report`.
    let buf = unsafe {
        core::slice::from_raw_parts(
            (report as *const BtdrvHidReport).cast::<u8>(),
            size_of::<BtdrvHidReport>(),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::WRITE_HID_DATA)
        .in_raw(in_bytes)
        .in_buffer(buf, BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE))
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// WriteHidData2 (cmd 20, in addr + HipcPointer in variable).
pub(crate) fn write_hid_data2(
    service: &Session,
    addr: BtdrvAddress,
    buf: &[u8],
) -> Result<(), DispatchError> {
    // SAFETY: `addr` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const addr).cast::<u8>(), size_of::<BtdrvAddress>())
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::WRITE_HID_DATA2)
        .in_raw(in_bytes)
        .in_buffer(buf, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// SetHidReport \[1.0.0-8.1.1\] (cmd 21, in struct + HipcPointer fixed in BtdrvHidData).
pub(crate) fn set_hid_report_legacy(
    service: &Session,
    input: SetHidReportIn,
    report: &BtdrvHidData,
) -> Result<(), DispatchError> {
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<SetHidReportIn>())
    };
    // SAFETY: `report` is a valid reference; viewing its bytes as a slice for
    // the IN buffer is sound, and the byte slice borrows `report`.
    let buf = unsafe {
        core::slice::from_raw_parts(
            (report as *const BtdrvHidData).cast::<u8>(),
            size_of::<BtdrvHidData>(),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::SET_HID_REPORT)
        .in_raw(in_bytes)
        .in_buffer(buf, BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE))
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// SetHidReport \[9.0.0+\] (cmd 21, in struct + HipcPointer fixed in BtdrvHidReport).
pub(crate) fn set_hid_report(
    service: &Session,
    input: SetHidReportIn,
    report: &BtdrvHidReport,
) -> Result<(), DispatchError> {
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<SetHidReportIn>())
    };
    // SAFETY: `report` is a valid reference; viewing its bytes as a slice for
    // the IN buffer is sound, and the byte slice borrows `report`.
    let buf = unsafe {
        core::slice::from_raw_parts(
            (report as *const BtdrvHidReport).cast::<u8>(),
            size_of::<BtdrvHidReport>(),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::SET_HID_REPORT)
        .in_raw(in_bytes)
        .in_buffer(buf, BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE))
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// GetHidReport (cmd 22).
pub(crate) fn get_hid_report(
    service: &Session,
    input: GetHidReportIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::GET_HID_REPORT, input)
}

/// TriggerConnection \[1.0.0-8.1.1\] (cmd 23).
pub(crate) fn trigger_connection_legacy(
    service: &Session,
    addr: BtdrvAddress,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::TRIGGER_CONNECTION, addr)
}

/// TriggerConnection \[9.0.0+\] (cmd 23).
pub(crate) fn trigger_connection(
    service: &Session,
    input: TriggerConnectionIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::TRIGGER_CONNECTION, input)
}

/// AddPairedDeviceInfo (cmd 24, HipcPointer fixed in).
pub(crate) fn add_paired_device_info(
    service: &Session,
    settings: &SetSysBluetoothDevicesSettings,
) -> Result<(), DispatchError> {
    dispatch_in_buf_ptr_fixed(service, settings, proto::ADD_PAIRED_DEVICE_INFO)
}

/// GetPairedDeviceInfo (cmd 25, in addr + HipcPointer fixed out).
pub(crate) fn get_paired_device_info(
    service: &Session,
    addr: BtdrvAddress,
    out: &mut SetSysBluetoothDevicesSettings,
) -> Result<(), DispatchError> {
    // SAFETY: `addr` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const addr).cast::<u8>(), size_of::<BtdrvAddress>())
    };
    // SAFETY: `out` is a valid mutable reference; viewing its bytes as a
    // slice for the OUT buffer is sound, and the byte slice borrows `out`.
    let out_buf = unsafe {
        core::slice::from_raw_parts_mut(
            (out as *mut SetSysBluetoothDevicesSettings).cast::<u8>(),
            size_of::<SetSysBluetoothDevicesSettings>(),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::GET_PAIRED_DEVICE_INFO)
        .in_raw(in_bytes)
        .out_buffer(out_buf, BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE))
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// FinalizeHid (cmd 26).
pub(crate) fn finalize_hid(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::FINALIZE_HID)
}

/// GetHidEventInfo (cmd 27, out u32 + HipcPointer out variable).
pub(crate) fn get_hid_event_info(service: &Session, buf: &mut [u8]) -> Result<u32, DispatchError> {
    dispatch_out_u32_out_buf(service, buf, proto::GET_HID_EVENT_INFO)
}

// Radio/Modulation (cmds 28-35)

/// SetTsi (cmd 28).
pub(crate) fn set_tsi(service: &Session, input: AddrU8In) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_TSI, input)
}

/// EnableBurstMode (cmd 29).
pub(crate) fn enable_burst_mode(service: &Session, input: AddrU8In) -> Result<(), DispatchError> {
    dispatch_in(service, proto::ENABLE_BURST_MODE, input)
}

/// SetZeroRetransmission (cmd 30, in addr + HipcPointer in variable).
pub(crate) fn set_zero_retransmission(
    service: &Session,
    addr: BtdrvAddress,
    report_ids: &[u8],
) -> Result<(), DispatchError> {
    // SAFETY: `addr` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const addr).cast::<u8>(), size_of::<BtdrvAddress>())
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::SET_ZERO_RETRANSMISSION)
        .in_raw(in_bytes)
        .in_buffer(report_ids, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// EnableMcMode (cmd 31).
pub(crate) fn enable_mc_mode(service: &Session, flag: u8) -> Result<(), DispatchError> {
    dispatch_in(service, proto::ENABLE_MC_MODE, flag)
}

/// EnableLlrScan (cmd 32).
pub(crate) fn enable_llr_scan(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::ENABLE_LLR_SCAN)
}

/// DisableLlrScan (cmd 33).
pub(crate) fn disable_llr_scan(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::DISABLE_LLR_SCAN)
}

/// EnableRadio (cmd 34).
pub(crate) fn enable_radio(service: &Session, flag: u8) -> Result<(), DispatchError> {
    dispatch_in(service, proto::ENABLE_RADIO, flag)
}

/// SetVisibility (cmd 35).
pub(crate) fn set_visibility(service: &Session, input: TwoBoolsIn) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_VISIBILITY, input)
}

// 4.0.0+ shifted commands (cmds 36-45)

/// EnableTbfcScan (cmd 36, 4.0.0+).
pub(crate) fn enable_tbfc_scan(service: &Session, flag: u8) -> Result<(), DispatchError> {
    dispatch_in(service, proto::ENABLE_TBFC_SCAN, flag)
}

/// RegisterHidReportEvent (cmd 37, 4.0.0+) — returns event handle.
pub(crate) fn register_hid_report_event(service: &Session) -> Result<u32, AcquireEventError> {
    acquire_event(service, proto::REGISTER_HID_REPORT_EVENT)
}

/// RegisterHidReportEvent \[pre-4.0.0\] (cmd 36) — returns event handle.
pub(crate) fn register_hid_report_event_legacy(
    service: &Session,
) -> Result<u32, AcquireEventError> {
    acquire_event(service, proto::REGISTER_HID_REPORT_EVENT_LEGACY)
}

/// GetHidReportEventInfo (cmd 38, 4.0.0+, out u32 + HipcPointer out variable).
pub(crate) fn get_hid_report_event_info(
    service: &Session,
    buf: &mut [u8],
) -> Result<u32, DispatchError> {
    dispatch_out_u32_out_buf(service, buf, proto::GET_HID_REPORT_EVENT_INFO)
}

/// GetHidReportEventInfo \[pre-4.0.0\] (cmd 37, out u32 + HipcPointer out variable).
pub(crate) fn get_hid_report_event_info_legacy(
    service: &Session,
    buf: &mut [u8],
) -> Result<u32, DispatchError> {
    dispatch_out_u32_out_buf(service, buf, proto::GET_HID_REPORT_EVENT_INFO_LEGACY)
}

/// GetHidReportEventInfo \[7.0.0+\] shared memory handle variant (cmd 38).
pub(crate) fn get_hid_report_event_shared_mem_handle(
    service: &Session,
) -> Result<u32, AcquireEventError> {
    acquire_event(service, proto::GET_HID_REPORT_EVENT_INFO)
}

/// GetLatestPlr \[pre-9.0.0\] (HipcMapAlias fixed out, PlrStatistics).
pub(crate) fn get_latest_plr_statistics(
    service: &Session,
    out: &mut BtdrvPlrStatistics,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_out_buf_alias_fixed(service, out, cmd_id)
}

/// GetLatestPlr \[9.0.0+\] (HipcMapAlias fixed out, PlrList).
pub(crate) fn get_latest_plr_list(
    service: &Session,
    out: &mut BtdrvPlrList,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_out_buf_alias_fixed(service, out, cmd_id)
}

/// GetPendingConnections (cmd 40 / 39 pre-4.0.0).
pub(crate) fn get_pending_connections(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    dispatch_no_io(service, cmd_id)
}

/// GetChannelMap (HipcMapAlias fixed out).
pub(crate) fn get_channel_map(
    service: &Session,
    out: &mut BtdrvChannelMapList,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_out_buf_alias_fixed(service, out, cmd_id)
}

/// EnableTxPowerBoostSetting.
pub(crate) fn enable_tx_power_boost_setting(
    service: &Session,
    flag: u8,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, flag)
}

/// IsTxPowerBoostSettingEnabled.
pub(crate) fn is_tx_power_boost_setting_enabled(
    service: &Session,
    cmd_id: u32,
) -> Result<u8, DispatchError> {
    dispatch_out(service, cmd_id)
}

/// EnableAfhSetting.
pub(crate) fn enable_afh_setting(
    service: &Session,
    flag: u8,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, flag)
}

/// IsAfhSettingEnabled.
pub(crate) fn is_afh_setting_enabled(service: &Session, cmd_id: u32) -> Result<u8, DispatchError> {
    dispatch_out(service, cmd_id)
}

// BLE (cmds 46-61)

/// InitializeBle (cmd 46) — returns event handle.
pub(crate) fn initialize_ble(service: &Session) -> Result<u32, AcquireEventError> {
    acquire_event(service, proto::INITIALIZE_BLE)
}

/// EnableBle (cmd 47).
pub(crate) fn enable_ble(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::ENABLE_BLE)
}

/// DisableBle (cmd 48).
pub(crate) fn disable_ble(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::DISABLE_BLE)
}

/// FinalizeBle (cmd 49).
pub(crate) fn finalize_ble(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::FINALIZE_BLE)
}

/// SetBleVisibility (cmd 50).
pub(crate) fn set_ble_visibility(
    service: &Session,
    input: TwoBoolsIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_BLE_VISIBILITY, input)
}

/// SetBleConnectionParameter \[5.0.0-8.1.1\] (cmd 51).
pub(crate) fn set_le_connection_parameter(
    service: &Session,
    param: BtdrvLeConnectionParams,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_BLE_CONNECTION_PARAMETER, param)
}

/// SetBleConnectionParameter \[9.0.0+\] (cmd 51).
pub(crate) fn set_ble_connection_parameter(
    service: &Session,
    input: SetBleConnectionParameterIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_BLE_CONNECTION_PARAMETER, input)
}

/// SetBleDefaultConnectionParameter \[5.0.0-8.1.1\] (cmd 52).
pub(crate) fn set_le_default_connection_parameter(
    service: &Session,
    param: BtdrvLeConnectionParams,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_BLE_DEFAULT_CONNECTION_PARAMETER, param)
}

/// SetBleDefaultConnectionParameter \[9.0.0+\] (cmd 52).
pub(crate) fn set_ble_default_connection_parameter(
    service: &Session,
    param: BtdrvBleConnectionParameter,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_BLE_DEFAULT_CONNECTION_PARAMETER, param)
}

/// SetBleAdvertiseData (cmd 53, HipcPointer fixed in).
pub(crate) fn set_ble_advertise_data(
    service: &Session,
    data: &BtdrvBleAdvertisePacketData,
) -> Result<(), DispatchError> {
    dispatch_in_buf_ptr_fixed(service, data, proto::SET_BLE_ADVERTISE_DATA)
}

/// SetBleAdvertiseParameter (cmd 54).
pub(crate) fn set_ble_advertise_parameter(
    service: &Session,
    input: SetBleAdvertiseParameterIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_BLE_ADVERTISE_PARAMETER, input)
}

/// StartBleScan (cmd 55).
pub(crate) fn start_ble_scan(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::START_BLE_SCAN)
}

/// StopBleScan (cmd 56).
pub(crate) fn stop_ble_scan(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::STOP_BLE_SCAN)
}

/// AddBleScanFilterCondition (cmd 57, HipcPointer fixed in).
pub(crate) fn add_ble_scan_filter_condition(
    service: &Session,
    filter: &BtdrvBleAdvertiseFilter,
) -> Result<(), DispatchError> {
    dispatch_in_buf_ptr_fixed(service, filter, proto::ADD_BLE_SCAN_FILTER_CONDITION)
}

/// DeleteBleScanFilterCondition (cmd 58, HipcPointer fixed in).
pub(crate) fn delete_ble_scan_filter_condition(
    service: &Session,
    filter: &BtdrvBleAdvertiseFilter,
) -> Result<(), DispatchError> {
    dispatch_in_buf_ptr_fixed(service, filter, proto::DELETE_BLE_SCAN_FILTER_CONDITION)
}

/// DeleteBleScanFilter (cmd 59).
pub(crate) fn delete_ble_scan_filter(service: &Session, index: u8) -> Result<(), DispatchError> {
    dispatch_in(service, proto::DELETE_BLE_SCAN_FILTER, index)
}

/// ClearBleScanFilters (cmd 60).
pub(crate) fn clear_ble_scan_filters(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::CLEAR_BLE_SCAN_FILTERS)
}

/// EnableBleScanFilter (cmd 61).
pub(crate) fn enable_ble_scan_filter(service: &Session, flag: u8) -> Result<(), DispatchError> {
    dispatch_in(service, proto::ENABLE_BLE_SCAN_FILTER, flag)
}

// GATT (cmds 62-97)

/// RegisterGattClient (cmd 62).
pub(crate) fn register_gatt_client(
    service: &Session,
    uuid: BtdrvGattAttributeUuid,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::REGISTER_GATT_CLIENT, uuid)
}

/// UnregisterGattClient (cmd 63).
pub(crate) fn unregister_gatt_client(
    service: &Session,
    client_if: u8,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::UNREGISTER_GATT_CLIENT, client_if)
}

/// UnregisterAllGattClients (cmd 64).
pub(crate) fn unregister_all_gatt_clients(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::UNREGISTER_ALL_GATT_CLIENTS)
}

/// ConnectGattServer (cmd 65).
pub(crate) fn connect_gatt_server(
    service: &Session,
    input: ConnectGattServerIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::CONNECT_GATT_SERVER, input)
}

/// CancelConnectGattServer (cmd 66, 5.1.0+).
pub(crate) fn cancel_connect_gatt_server(
    service: &Session,
    input: CancelConnectGattServerIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::CANCEL_CONNECT_GATT_SERVER, input)
}

/// DisconnectGattServer.
pub(crate) fn disconnect_gatt_server(
    service: &Session,
    conn_id: u32,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, conn_id)
}

/// GetGattAttribute \[pre-9.0.0\].
pub(crate) fn get_gatt_attribute_legacy(
    service: &Session,
    input: GetGattAttributeLegacyIn,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, input)
}

/// GetGattAttribute \[9.0.0+\].
pub(crate) fn get_gatt_attribute(
    service: &Session,
    conn_id: u32,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, conn_id)
}

/// GetGattService.
pub(crate) fn get_gatt_service(
    service: &Session,
    input: GetGattServiceIn,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, input)
}

/// ConfigureAttMtu.
pub(crate) fn configure_att_mtu(
    service: &Session,
    input: ConfigureAttMtuIn,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, input)
}

/// RegisterGattServer.
pub(crate) fn register_gatt_server(
    service: &Session,
    uuid: BtdrvGattAttributeUuid,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, uuid)
}

/// UnregisterGattServer.
pub(crate) fn unregister_gatt_server(
    service: &Session,
    server_if: u8,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, server_if)
}

/// ConnectGattClient.
pub(crate) fn connect_gatt_client(
    service: &Session,
    input: ConnectGattClientIn,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, input)
}

/// DisconnectGattClient \[pre-9.0.0\].
pub(crate) fn disconnect_gatt_client_legacy(
    service: &Session,
    input: DisconnectGattClientLegacyIn,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, input)
}

/// DisconnectGattClient \[9.0.0+\].
pub(crate) fn disconnect_gatt_client(
    service: &Session,
    conn_id: u8,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, conn_id)
}

/// AddGattService (cmd 75).
pub(crate) fn add_gatt_service(
    service: &Session,
    input: AddGattServiceIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::ADD_GATT_SERVICE, input)
}

/// EnableGattService.
pub(crate) fn enable_gatt_service(
    service: &Session,
    input: EnableGattServiceIn,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, input)
}

/// AddGattCharacteristic (cmd 77).
pub(crate) fn add_gatt_characteristic(
    service: &Session,
    input: AddGattCharacteristicIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::ADD_GATT_CHARACTERISTIC, input)
}

/// AddGattDescriptor.
pub(crate) fn add_gatt_descriptor(
    service: &Session,
    input: AddGattDescriptorIn,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, input)
}

/// GetBleManagedEventInfo (out u32 + HipcPointer out variable).
pub(crate) fn get_ble_managed_event_info(
    service: &Session,
    buf: &mut [u8],
    cmd_id: u32,
) -> Result<u32, DispatchError> {
    dispatch_out_u32_out_buf(service, buf, cmd_id)
}

/// GetGattFirstCharacteristic.
pub(crate) fn get_gatt_first_characteristic(
    service: &Session,
    input: GetGattFirstCharacteristicIn,
    cmd_id: u32,
) -> Result<GetGattCharacteristicOut, DispatchError> {
    dispatch_in_out(service, cmd_id, input)
}

/// GetGattNextCharacteristic.
pub(crate) fn get_gatt_next_characteristic(
    service: &Session,
    input: GetGattNextCharacteristicIn,
    cmd_id: u32,
) -> Result<GetGattCharacteristicOut, DispatchError> {
    dispatch_in_out(service, cmd_id, input)
}

/// GetGattFirstDescriptor.
pub(crate) fn get_gatt_first_descriptor(
    service: &Session,
    input: GetGattFirstDescriptorIn,
    cmd_id: u32,
) -> Result<BtdrvGattId, DispatchError> {
    dispatch_in_out(service, cmd_id, input)
}

/// GetGattNextDescriptor.
pub(crate) fn get_gatt_next_descriptor(
    service: &Session,
    input: GetGattNextDescriptorIn,
    cmd_id: u32,
) -> Result<BtdrvGattId, DispatchError> {
    dispatch_in_out(service, cmd_id, input)
}

/// RegisterGattManagedDataPath (cmd 84).
pub(crate) fn register_gatt_managed_data_path(
    service: &Session,
    uuid: BtdrvGattAttributeUuid,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::REGISTER_GATT_MANAGED_DATA_PATH, uuid)
}

/// UnregisterGattManagedDataPath (cmd 85).
pub(crate) fn unregister_gatt_managed_data_path(
    service: &Session,
    uuid: BtdrvGattAttributeUuid,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::UNREGISTER_GATT_MANAGED_DATA_PATH, uuid)
}

/// RegisterGattHidDataPath (cmd 86).
pub(crate) fn register_gatt_hid_data_path(
    service: &Session,
    uuid: BtdrvGattAttributeUuid,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::REGISTER_GATT_HID_DATA_PATH, uuid)
}

/// UnregisterGattHidDataPath (cmd 87).
pub(crate) fn unregister_gatt_hid_data_path(
    service: &Session,
    uuid: BtdrvGattAttributeUuid,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::UNREGISTER_GATT_HID_DATA_PATH, uuid)
}

/// RegisterGattDataPath (cmd 88).
pub(crate) fn register_gatt_data_path(
    service: &Session,
    uuid: BtdrvGattAttributeUuid,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::REGISTER_GATT_DATA_PATH, uuid)
}

/// UnregisterGattDataPath.
pub(crate) fn unregister_gatt_data_path(
    service: &Session,
    uuid: BtdrvGattAttributeUuid,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, uuid)
}

/// ReadGattCharacteristic.
pub(crate) fn read_gatt_characteristic(
    service: &Session,
    input: ReadGattCharacteristicIn,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, input)
}

/// ReadGattDescriptor.
pub(crate) fn read_gatt_descriptor(
    service: &Session,
    input: ReadGattDescriptorIn,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, input)
}

/// WriteGattCharacteristic (in struct + HipcPointer in variable).
pub(crate) fn write_gatt_characteristic(
    service: &Session,
    input: WriteGattCharacteristicIn,
    buf: &[u8],
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<WriteGattCharacteristicIn>(),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .in_buffer(buf, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// WriteGattDescriptor (in struct + HipcPointer in variable).
pub(crate) fn write_gatt_descriptor(
    service: &Session,
    input: WriteGattDescriptorIn,
    buf: &[u8],
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<WriteGattDescriptorIn>(),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .in_buffer(buf, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// RegisterGattNotification (cmd 94).
pub(crate) fn register_gatt_notification(
    service: &Session,
    input: GattNotificationIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::REGISTER_GATT_NOTIFICATION, input)
}

/// UnregisterGattNotification.
pub(crate) fn unregister_gatt_notification(
    service: &Session,
    input: GattNotificationIn,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, input)
}

/// GetLeHidEventInfo (out u32 + HipcPointer out variable).
pub(crate) fn get_le_hid_event_info(
    service: &Session,
    buf: &mut [u8],
    cmd_id: u32,
) -> Result<u32, DispatchError> {
    dispatch_out_u32_out_buf(service, buf, cmd_id)
}

/// RegisterBleHidEvent — returns event handle.
pub(crate) fn register_ble_hid_event(
    service: &Session,
    cmd_id: u32,
) -> Result<u32, AcquireEventError> {
    acquire_event(service, cmd_id)
}

/// SetBleScanParameter (cmd 98).
pub(crate) fn set_ble_scan_parameter(
    service: &Session,
    input: SetBleScanParameterIn,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_BLE_SCAN_PARAMETER, input)
}

// Other (cmds 99-100)

/// MoveToSecondaryPiconet (cmd 99, 10.0.0+).
pub(crate) fn move_to_secondary_piconet(
    service: &Session,
    addr: BtdrvAddress,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::MOVE_TO_SECONDARY_PICONET, addr)
}

/// IsBluetoothEnabled (cmd 100, 12.0.0+).
pub(crate) fn is_bluetooth_enabled(service: &Session) -> Result<u8, DispatchError> {
    dispatch_out(service, proto::IS_BLUETOOTH_ENABLED)
}

// Audio (cmds 128-149)

/// AcquireAudioEvent (cmd 128) — returns event handle.
pub(crate) fn acquire_audio_event(service: &Session) -> Result<u32, AcquireEventError> {
    acquire_event(service, proto::ACQUIRE_AUDIO_EVENT)
}

/// GetAudioEventInfo (cmd 129, out u32 + HipcPointer out variable).
pub(crate) fn get_audio_event_info(
    service: &Session,
    buf: &mut [u8],
) -> Result<u32, DispatchError> {
    dispatch_out_u32_out_buf(service, buf, proto::GET_AUDIO_EVENT_INFO)
}

/// OpenAudioConnection (cmd 130).
pub(crate) fn open_audio_connection(
    service: &Session,
    addr: BtdrvAddress,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::OPEN_AUDIO_CONNECTION, addr)
}

/// CloseAudioConnection (cmd 131).
pub(crate) fn close_audio_connection(
    service: &Session,
    addr: BtdrvAddress,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::CLOSE_AUDIO_CONNECTION, addr)
}

/// OpenAudioOut (cmd 132).
pub(crate) fn open_audio_out(service: &Session, addr: BtdrvAddress) -> Result<u32, DispatchError> {
    dispatch_in_out(service, proto::OPEN_AUDIO_OUT, addr)
}

/// CloseAudioOut (cmd 133).
pub(crate) fn close_audio_out(service: &Session, audio_handle: u32) -> Result<(), DispatchError> {
    dispatch_in(service, proto::CLOSE_AUDIO_OUT, audio_handle)
}

/// AcquireAudioOutStateChangedEvent (cmd 134) — returns event handle.
pub(crate) fn acquire_audio_out_state_changed_event(
    service: &Session,
    audio_handle: u32,
) -> Result<u32, AcquireEventError> {
    acquire_event_with_u32_in(
        service,
        proto::ACQUIRE_AUDIO_OUT_STATE_CHANGED_EVENT,
        audio_handle,
    )
}

/// StartAudioOut (cmd 135).
pub(crate) fn start_audio_out(
    service: &Session,
    input: StartAudioOutIn,
) -> Result<StartAudioOutOut, DispatchError> {
    dispatch_in_out(service, proto::START_AUDIO_OUT, input)
}

/// StopAudioOut (cmd 136).
pub(crate) fn stop_audio_out(service: &Session, audio_handle: u32) -> Result<(), DispatchError> {
    dispatch_in(service, proto::STOP_AUDIO_OUT, audio_handle)
}

/// GetAudioOutState (cmd 137).
pub(crate) fn get_audio_out_state(
    service: &Session,
    audio_handle: u32,
) -> Result<u32, DispatchError> {
    dispatch_in_out(service, proto::GET_AUDIO_OUT_STATE, audio_handle)
}

/// GetAudioOutFeedingCodec (cmd 138).
pub(crate) fn get_audio_out_feeding_codec(
    service: &Session,
    audio_handle: u32,
) -> Result<u32, DispatchError> {
    dispatch_in_out(service, proto::GET_AUDIO_OUT_FEEDING_CODEC, audio_handle)
}

/// GetAudioOutFeedingParameter (cmd 139).
pub(crate) fn get_audio_out_feeding_parameter(
    service: &Session,
    audio_handle: u32,
) -> Result<BtdrvPcmParameter, DispatchError> {
    dispatch_in_out(
        service,
        proto::GET_AUDIO_OUT_FEEDING_PARAMETER,
        audio_handle,
    )
}

/// AcquireAudioOutBufferAvailableEvent (cmd 140) — returns event handle.
pub(crate) fn acquire_audio_out_buffer_available_event(
    service: &Session,
    audio_handle: u32,
) -> Result<u32, AcquireEventError> {
    acquire_event_with_u32_in(
        service,
        proto::ACQUIRE_AUDIO_OUT_BUFFER_AVAILABLE_EVENT,
        audio_handle,
    )
}

/// SendAudioData (cmd 141, in u32 + out u64 + HipcPointer in variable).
pub(crate) fn send_audio_data(
    service: &Session,
    audio_handle: u32,
    buf: &[u8],
) -> Result<u64, DispatchError> {
    // SAFETY: `audio_handle` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const audio_handle).cast::<u8>(), size_of::<u32>())
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::SEND_AUDIO_DATA)
        .in_raw(in_bytes)
        .out_size(size_of::<u64>())
        .in_buffer(buf, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<u64>() bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) })
}

/// AcquireAudioControlInputStateChangedEvent (cmd 142) — returns event handle.
pub(crate) fn acquire_audio_control_input_state_changed_event(
    service: &Session,
) -> Result<u32, AcquireEventError> {
    acquire_event(
        service,
        proto::ACQUIRE_AUDIO_CONTROL_INPUT_STATE_CHANGED_EVENT,
    )
}

/// GetAudioControlInputState (cmd 143, out u32 + HipcPointer out variable).
pub(crate) fn get_audio_control_input_state(
    service: &Session,
    buf: &mut [u8],
) -> Result<u32, DispatchError> {
    dispatch_out_u32_out_buf(service, buf, proto::GET_AUDIO_CONTROL_INPUT_STATE)
}

/// AcquireAudioConnectionStateChangedEvent (cmd 144) — returns event handle.
pub(crate) fn acquire_audio_connection_state_changed_event(
    service: &Session,
) -> Result<u32, AcquireEventError> {
    acquire_event(service, proto::ACQUIRE_AUDIO_CONNECTION_STATE_CHANGED_EVENT)
}

/// GetConnectedAudioDevice (cmd 145, out u32 + HipcPointer out variable).
pub(crate) fn get_connected_audio_device(
    service: &Session,
    buf: &mut [u8],
) -> Result<u32, DispatchError> {
    dispatch_out_u32_out_buf(service, buf, proto::GET_CONNECTED_AUDIO_DEVICE)
}

/// CloseAudioControlInput (cmd 146).
pub(crate) fn close_audio_control_input(
    service: &Session,
    addr: BtdrvAddress,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::CLOSE_AUDIO_CONTROL_INPUT, addr)
}

/// RegisterAudioControlNotification (cmd 147).
pub(crate) fn register_audio_control_notification(
    service: &Session,
    input: AddrU32In,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::REGISTER_AUDIO_CONTROL_NOTIFICATION, input)
}

/// SendAudioControlPassthroughCommand (cmd 148).
pub(crate) fn send_audio_control_passthrough_command(
    service: &Session,
    input: AddrU32U32In,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::SEND_AUDIO_CONTROL_PASSTHROUGH_COMMAND,
        input,
    )
}

/// SendAudioControlSetAbsoluteVolumeCommand (cmd 149).
pub(crate) fn send_audio_control_set_absolute_volume_command(
    service: &Session,
    input: AddrU32In,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::SEND_AUDIO_CONTROL_SET_ABSOLUTE_VOLUME_COMMAND,
        input,
    )
}

// Debug (cmds 256-258)

/// IsManufacturingMode (cmd 256).
pub(crate) fn is_manufacturing_mode(service: &Session) -> Result<u8, DispatchError> {
    dispatch_out(service, proto::IS_MANUFACTURING_MODE)
}

/// EmulateBluetoothCrash (cmd 257).
pub(crate) fn emulate_bluetooth_crash(service: &Session, reason: u32) -> Result<(), DispatchError> {
    dispatch_in(service, proto::EMULATE_BLUETOOTH_CRASH, reason)
}

/// GetBleChannelMap (cmd 258, HipcMapAlias fixed out).
pub(crate) fn get_ble_channel_map(
    service: &Session,
    out: &mut BtdrvChannelMapList,
) -> Result<(), DispatchError> {
    dispatch_out_buf_alias_fixed(service, out, proto::GET_BLE_CHANNEL_MAP)
}

// Shared dispatch helpers

/// HipcPointer fixed-size input buffer.
pub(crate) fn dispatch_in_buf_ptr_fixed<T>(
    service: &Session,
    input: &T,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `input` is a valid reference; viewing its bytes as a slice for
    // the IN buffer is sound, and the byte slice borrows `input`.
    let buf =
        unsafe { core::slice::from_raw_parts((input as *const T).cast::<u8>(), size_of::<T>()) };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_buffer(buf, BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE))
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// HipcPointer fixed-size output buffer.
fn dispatch_out_buf_ptr_fixed<T>(
    service: &Session,
    out: &mut T,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `out` is a valid mutable reference; viewing its bytes as a slice
    // for the OUT buffer is sound, and the byte slice borrows `out`.
    let buf =
        unsafe { core::slice::from_raw_parts_mut((out as *mut T).cast::<u8>(), size_of::<T>()) };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .out_buffer(buf, BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE))
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// HipcMapAlias fixed-size output buffer.
fn dispatch_out_buf_alias_fixed<T>(
    service: &Session,
    out: &mut T,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `out` is a valid mutable reference; viewing its bytes as a slice
    // for the OUT buffer is sound, and the byte slice borrows `out`.
    let buf =
        unsafe { core::slice::from_raw_parts_mut((out as *mut T).cast::<u8>(), size_of::<T>()) };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .out_buffer(buf, BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::FIXED_SIZE))
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Dispatches a command that returns a copy handle for an event.
pub(crate) fn acquire_event(service: &Session, cmd_id: u32) -> Result<u32, AcquireEventError> {
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

/// Dispatches a command with u32 input that returns a copy handle for an event.
fn acquire_event_with_u32_in(
    service: &Session,
    cmd_id: u32,
    input: u32,
) -> Result<u32, AcquireEventError> {
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<u32>()) };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AcquireEventError::Dispatch)?;

    let Some(handle) = result.copy_handles.first().copied() else {
        return Err(AcquireEventError::MissingHandle);
    };

    Ok(handle)
}

/// Dispatches a command with u16 input that returns a copy handle.
fn dispatch_in_u16_get_handle(
    service: &Session,
    cmd_id: u32,
    input: u16,
) -> Result<u32, AcquireEventError> {
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<u16>()) };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AcquireEventError::Dispatch)?;

    let Some(handle) = result.copy_handles.first().copied() else {
        return Err(AcquireEventError::MissingHandle);
    };

    Ok(handle)
}

/// Dispatches a command with out u32 + HipcPointer out buffer.
fn dispatch_out_u32_out_buf(
    service: &Session,
    buf: &mut [u8],
    cmd_id: u32,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<u32>())
        .out_buffer(buf, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<u32>() bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}

// Error types

/// Error returned by event acquisition commands (copy handle only).
#[derive(Debug, thiserror::Error)]
pub enum AcquireEventError {
    #[error("failed to dispatch event acquisition")]
    Dispatch(#[source] DispatchError),
    #[error("event acquisition response did not include expected copy handle")]
    MissingHandle,
}
