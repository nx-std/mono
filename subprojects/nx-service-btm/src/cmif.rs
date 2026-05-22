//! CMIF protocol operations for the Bluetooth Manager service.

use core::{mem::size_of, ptr};

use nx_sf::service::{BufferAttr, DispatchError, OutHandleAttr, Session};

use crate::{
    dispatch::{dispatch_in, dispatch_in_out, dispatch_no_io, dispatch_out},
    proto,
    types::{
        AddrBoolIn, BlePairDeviceIn, BleUnpairDeviceIn, BtdrvAddress,
        BtdrvBleAdvertisePacketParameter, BtdrvBleConnectionInfo, BtdrvBleScanResult,
        BtdrvGattAttributeUuid, BtmBleDataPath, BtmConnectedDeviceV13, BtmDeviceConditionV100,
        BtmDeviceConditionV510, BtmDeviceConditionV800, BtmDeviceConditionV900, BtmDeviceInfoList,
        BtmDeviceInfoV1, BtmDeviceInfoV13, BtmDeviceSlotModeList, BtmGattCharacteristic,
        BtmGattClientConditionList, BtmGattDescriptor, BtmGattService, BtmHostDevicePropertyV1,
        BtmHostDevicePropertyV13, BtmZeroRetransmissionList, ConfigureBleMtuIn, GetGattServiceIn,
        HandleConnectionIn, LlrNotifyIn, RegisterAruidIn,
    },
};

// ---------------------------------------------------------------------------
// Core commands (cmd 0-17, always the same IDs)
// ---------------------------------------------------------------------------

/// GetState (cmd 0).
pub(crate) fn get_state(service: &Session) -> Result<u32, DispatchError> {
    dispatch_out(service, proto::GET_STATE)
}

/// GetHostDeviceProperty \[1.0.0-12.1.0\] (cmd 1, out raw).
pub(crate) fn get_host_device_property_legacy(
    service: &Session,
) -> Result<BtmHostDevicePropertyV1, DispatchError> {
    dispatch_out(service, proto::GET_HOST_DEVICE_PROPERTY)
}

/// GetHostDeviceProperty \[13.0.0+\] (cmd 1, HipcPointer fixed out).
pub(crate) fn get_host_device_property(
    service: &Session,
    out: &mut BtmHostDevicePropertyV13,
) -> Result<(), DispatchError> {
    dispatch_out_buf_ptr_fixed(service, out, proto::GET_HOST_DEVICE_PROPERTY)
}

/// AcquireDeviceConditionEvent (cmd 2, pre-3.0.0 — no out flag).
pub(crate) fn acquire_device_condition_event_legacy(
    service: &Session,
) -> Result<u32, AcquireEventError> {
    acquire_event(service, proto::ACQUIRE_DEVICE_CONDITION_EVENT)
}

/// AcquireDeviceConditionEvent (cmd 2, 3.0.0+ — with out flag).
pub(crate) fn acquire_device_condition_event(
    service: &Session,
) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_DEVICE_CONDITION_EVENT)
}

/// GetDeviceCondition \[1.0.0-5.0.2\] (cmd 3, HipcPointer fixed out).
pub(crate) fn get_device_condition_v100(
    service: &Session,
    out: &mut BtmDeviceConditionV100,
) -> Result<(), DispatchError> {
    dispatch_out_buf_ptr_fixed(service, out, proto::GET_DEVICE_CONDITION)
}

/// GetDeviceCondition \[5.1.0-7.0.1\] (cmd 3, HipcPointer fixed out).
pub(crate) fn get_device_condition_v510(
    service: &Session,
    out: &mut BtmDeviceConditionV510,
) -> Result<(), DispatchError> {
    dispatch_out_buf_ptr_fixed(service, out, proto::GET_DEVICE_CONDITION)
}

/// GetDeviceCondition \[8.0.0-8.1.1\] (cmd 3, HipcPointer fixed out).
pub(crate) fn get_device_condition_v800(
    service: &Session,
    out: &mut BtmDeviceConditionV800,
) -> Result<(), DispatchError> {
    dispatch_out_buf_ptr_fixed(service, out, proto::GET_DEVICE_CONDITION)
}

/// GetDeviceCondition \[9.0.0-12.1.0\] (cmd 3, HipcPointer fixed out).
pub(crate) fn get_device_condition_v900(
    service: &Session,
    out: &mut BtmDeviceConditionV900,
) -> Result<(), DispatchError> {
    dispatch_out_buf_ptr_fixed(service, out, proto::GET_DEVICE_CONDITION)
}

/// GetDeviceCondition \[13.0.0+\] (cmd 3, in profile + HipcPointer out array).
pub(crate) fn get_device_condition(
    service: &Session,
    profile: u32,
    out: &mut [BtmConnectedDeviceV13],
) -> Result<i32, DispatchError> {
    // SAFETY: `profile` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const profile).cast::<u8>(), size_of::<u32>()) };
    // SAFETY: `out` is a valid `&mut [BtmConnectedDeviceV13]`; viewing it as
    // a byte slice for the OUT buffer is sound, and the byte slice borrows `out`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(out.as_mut_ptr().cast::<u8>(), core::mem::size_of_val(out))
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::GET_DEVICE_CONDITION)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<i32>() bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

/// SetBurstMode (cmd 4).
pub(crate) fn set_burst_mode(
    service: &Session,
    addr: &BtdrvAddress,
    flag: bool,
) -> Result<(), DispatchError> {
    let input = AddrBoolIn {
        addr: *addr,
        flag: u8::from(flag),
    };
    dispatch_in(service, proto::SET_BURST_MODE, input)
}

/// SetSlotMode (cmd 5).
pub(crate) fn set_slot_mode(
    service: &Session,
    list: &BtmDeviceSlotModeList,
) -> Result<(), DispatchError> {
    dispatch_in_buf_ptr_fixed(service, list, proto::SET_SLOT_MODE)
}

/// SetBluetoothMode (cmd 6, pre-9.0.0).
pub(crate) fn set_bluetooth_mode(service: &Session, mode: u32) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_BLUETOOTH_MODE, mode)
}

/// SetWlanMode (cmd 7).
pub(crate) fn set_wlan_mode(service: &Session, mode: u32) -> Result<(), DispatchError> {
    dispatch_in(service, proto::SET_WLAN_MODE, mode)
}

/// AcquireDeviceInfoEvent (cmd 8, pre-3.0.0 — no out flag).
pub(crate) fn acquire_device_info_event_legacy(
    service: &Session,
) -> Result<u32, AcquireEventError> {
    acquire_event(service, proto::ACQUIRE_DEVICE_INFO_EVENT)
}

/// AcquireDeviceInfoEvent (cmd 8, 3.0.0+ — with out flag).
pub(crate) fn acquire_device_info_event(
    service: &Session,
) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_DEVICE_INFO_EVENT)
}

/// GetDeviceInfo \[1.0.0-12.1.0\] (cmd 9, HipcPointer fixed out).
pub(crate) fn get_device_info_legacy(
    service: &Session,
    out: &mut BtmDeviceInfoList,
) -> Result<(), DispatchError> {
    dispatch_out_buf_ptr_fixed(service, out, proto::GET_DEVICE_INFO)
}

/// GetDeviceInfo \[13.0.0+\] (cmd 9, in profile + HipcPointer out array).
pub(crate) fn get_device_info(
    service: &Session,
    profile: u32,
    out: &mut [BtmDeviceInfoV13],
) -> Result<i32, DispatchError> {
    // SAFETY: `profile` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const profile).cast::<u8>(), size_of::<u32>()) };
    // SAFETY: `out` is a valid `&mut [BtmDeviceInfoV13]`; viewing it as
    // a byte slice for the OUT buffer is sound, and the byte slice borrows `out`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(out.as_mut_ptr().cast::<u8>(), core::mem::size_of_val(out))
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::GET_DEVICE_INFO)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<i32>() bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) })
}

/// AddDeviceInfo \[1.0.0-12.1.0\] (cmd 10, in raw).
pub(crate) fn add_device_info_legacy(
    service: &Session,
    info: &BtmDeviceInfoV1,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::ADD_DEVICE_INFO, *info)
}

/// AddDeviceInfo \[13.0.0+\] (cmd 10, HipcPointer fixed in).
pub(crate) fn add_device_info(
    service: &Session,
    info: &BtmDeviceInfoV13,
) -> Result<(), DispatchError> {
    dispatch_in_buf_ptr_fixed(service, info, proto::ADD_DEVICE_INFO)
}

/// RemoveDeviceInfo (cmd 11).
pub(crate) fn remove_device_info(
    service: &Session,
    addr: &BtdrvAddress,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::REMOVE_DEVICE_INFO, *addr)
}

/// IncreaseDeviceInfoOrder (cmd 12).
pub(crate) fn increase_device_info_order(
    service: &Session,
    addr: &BtdrvAddress,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::INCREASE_DEVICE_INFO_ORDER, *addr)
}

/// LlrNotify \[pre-9.0.0\] (cmd 13, in address only).
pub(crate) fn llr_notify_legacy(
    service: &Session,
    addr: &BtdrvAddress,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::LLR_NOTIFY, *addr)
}

/// LlrNotify \[9.0.0+\] (cmd 13, in address + unk).
pub(crate) fn llr_notify(
    service: &Session,
    addr: &BtdrvAddress,
    unk: i32,
) -> Result<(), DispatchError> {
    let input = LlrNotifyIn {
        addr: *addr,
        pad: [0; 2],
        unk,
    };
    dispatch_in(service, proto::LLR_NOTIFY, input)
}

/// EnableRadio (cmd 14).
pub(crate) fn enable_radio(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::ENABLE_RADIO)
}

/// DisableRadio (cmd 15).
pub(crate) fn disable_radio(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::DISABLE_RADIO)
}

/// HidDisconnect (cmd 16).
pub(crate) fn hid_disconnect(service: &Session, addr: &BtdrvAddress) -> Result<(), DispatchError> {
    dispatch_in(service, proto::HID_DISCONNECT, *addr)
}

/// HidSetRetransmissionMode (cmd 17).
pub(crate) fn hid_set_retransmission_mode(
    service: &Session,
    addr: &BtdrvAddress,
    list: &BtmZeroRetransmissionList,
) -> Result<(), DispatchError> {
    // SAFETY: `addr` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const *addr).cast::<u8>(), size_of::<BtdrvAddress>())
    };
    // SAFETY: `list` is a valid reference; viewing its bytes as a slice for
    // the IN buffer is sound, and the byte slice borrows `list`.
    let list_bytes = unsafe {
        core::slice::from_raw_parts(
            (list as *const BtmZeroRetransmissionList).cast::<u8>(),
            size_of::<BtmZeroRetransmissionList>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::HID_SET_RETRANSMISSION_MODE)
        .in_raw(in_bytes)
        .in_buffer(
            list_bytes,
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// Extended commands (cmd 18-22)
// ---------------------------------------------------------------------------

/// AcquireAwakeReqEvent (cmd 18, pre-3.0.0 — no out flag).
pub(crate) fn acquire_awake_req_event_legacy(service: &Session) -> Result<u32, AcquireEventError> {
    acquire_event(service, proto::ACQUIRE_AWAKE_REQ_EVENT)
}

/// AcquireAwakeReqEvent (cmd 18, 3.0.0+ — with out flag).
pub(crate) fn acquire_awake_req_event(service: &Session) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_AWAKE_REQ_EVENT)
}

/// AcquireLlrStateEvent (cmd 19, 4.0.0+).
pub(crate) fn acquire_llr_state_event(service: &Session) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_LLR_STATE_EVENT)
}

/// IsLlrStarted (cmd 20, 4.0.0+).
pub(crate) fn is_llr_started(service: &Session) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(service, proto::IS_LLR_STARTED)?;
    Ok(val & 1 != 0)
}

/// EnableSlotSaving (cmd 21, 4.0.0+).
pub(crate) fn enable_slot_saving(service: &Session, flag: bool) -> Result<(), DispatchError> {
    dispatch_in(service, proto::ENABLE_SLOT_SAVING, u8::from(flag))
}

/// ProtectDeviceInfo (cmd 22, 5.0.0+).
pub(crate) fn protect_device_info(
    service: &Session,
    addr: &BtdrvAddress,
    flag: bool,
) -> Result<(), DispatchError> {
    let input = AddrBoolIn {
        addr: *addr,
        flag: u8::from(flag),
    };
    dispatch_in(service, proto::PROTECT_DEVICE_INFO, input)
}

// ---------------------------------------------------------------------------
// BLE scan commands (5.0.0+/5.1.0+)
// ---------------------------------------------------------------------------

/// AcquireBleScanEvent (cmd 23, 5.0.0+).
pub(crate) fn acquire_ble_scan_event(service: &Session) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_BLE_SCAN_EVENT)
}

/// GetBleScanParameterGeneral (cmd 24, 5.1.0+).
pub(crate) fn get_ble_scan_parameter_general(
    service: &Session,
    parameter_id: u16,
) -> Result<BtdrvBleAdvertisePacketParameter, DispatchError> {
    dispatch_in_out(service, proto::GET_BLE_SCAN_PARAMETER_GENERAL, parameter_id)
}

/// GetBleScanParameterSmartDevice (cmd 25, 5.1.0+).
pub(crate) fn get_ble_scan_parameter_smart_device(
    service: &Session,
    parameter_id: u16,
) -> Result<BtdrvGattAttributeUuid, DispatchError> {
    dispatch_in_out(
        service,
        proto::GET_BLE_SCAN_PARAMETER_SMART_DEVICE,
        parameter_id,
    )
}

/// StartBleScanForGeneral (cmd 26, 5.1.0+).
pub(crate) fn start_ble_scan_for_general(
    service: &Session,
    param: &BtdrvBleAdvertisePacketParameter,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::START_BLE_SCAN_FOR_GENERAL, *param)
}

/// StopBleScanForGeneral (cmd 27, 5.1.0+).
pub(crate) fn stop_ble_scan_for_general(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::STOP_BLE_SCAN_FOR_GENERAL)
}

/// GetBleScanResultsForGeneral (cmd 28, 5.1.0+).
pub(crate) fn get_ble_scan_results_for_general(
    service: &Session,
    results: &mut [BtdrvBleScanResult],
) -> Result<u8, DispatchError> {
    get_ble_scan_results(service, results, proto::GET_BLE_SCAN_RESULTS_FOR_GENERAL)
}

/// StartBleScanForPaired (cmd 29, 5.1.0+).
pub(crate) fn start_ble_scan_for_paired(
    service: &Session,
    param: &BtdrvBleAdvertisePacketParameter,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::START_BLE_SCAN_FOR_PAIRED, *param)
}

/// StopBleScanForPaired (cmd 30, 5.1.0+).
pub(crate) fn stop_ble_scan_for_paired(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::STOP_BLE_SCAN_FOR_PAIRED)
}

/// StartBleScanForSmartDevice (cmd 31, 5.1.0+).
pub(crate) fn start_ble_scan_for_smart_device(
    service: &Session,
    uuid: &BtdrvGattAttributeUuid,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::START_BLE_SCAN_FOR_SMART_DEVICE, *uuid)
}

/// StopBleScanForSmartDevice (cmd 32, 5.1.0+).
pub(crate) fn stop_ble_scan_for_smart_device(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::STOP_BLE_SCAN_FOR_SMART_DEVICE)
}

/// GetBleScanResultsForSmartDevice (cmd 33, 5.1.0+).
pub(crate) fn get_ble_scan_results_for_smart_device(
    service: &Session,
    results: &mut [BtdrvBleScanResult],
) -> Result<u8, DispatchError> {
    get_ble_scan_results(
        service,
        results,
        proto::GET_BLE_SCAN_RESULTS_FOR_SMART_DEVICE,
    )
}

// ---------------------------------------------------------------------------
// BLE connection commands (5.0.0+/5.1.0+ with cmd ID remapping)
// ---------------------------------------------------------------------------

/// AcquireBleConnectionEvent (cmd 34, 5.1.0+).
pub(crate) fn acquire_ble_connection_event(
    service: &Session,
) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_BLE_CONNECTION_EVENT)
}

/// BleConnect (cmd 35, 5.1.0+).
pub(crate) fn ble_connect(service: &Session, addr: &BtdrvAddress) -> Result<(), DispatchError> {
    dispatch_in(service, proto::BLE_CONNECT, *addr)
}

/// BleConnect \[5.0.0-5.0.2\] (cmd 24).
pub(crate) fn ble_connect_legacy(
    service: &Session,
    addr: &BtdrvAddress,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::BLE_CONNECT_LEGACY, *addr)
}

/// BleOverrideConnection (cmd 36, 5.1.0+).
pub(crate) fn ble_override_connection(service: &Session, id: u32) -> Result<(), DispatchError> {
    dispatch_in(service, proto::BLE_OVERRIDE_CONNECTION, id)
}

/// BleDisconnect (cmd 37, 5.1.0+).
pub(crate) fn ble_disconnect(
    service: &Session,
    connection_handle: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::BLE_DISCONNECT, connection_handle)
}

/// BleDisconnect \[5.0.0-5.0.2\] (cmd 25).
pub(crate) fn ble_disconnect_legacy(
    service: &Session,
    connection_handle: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::BLE_DISCONNECT_LEGACY, connection_handle)
}

/// BleGetConnectionState (cmd 38, 5.1.0+).
pub(crate) fn ble_get_connection_state(
    service: &Session,
    info: &mut [BtdrvBleConnectionInfo],
) -> Result<u8, DispatchError> {
    ble_get_connection_state_impl(service, info, proto::BLE_GET_CONNECTION_STATE)
}

/// BleGetConnectionState \[5.0.0-5.0.2\] (cmd 26).
pub(crate) fn ble_get_connection_state_legacy(
    service: &Session,
    info: &mut [BtdrvBleConnectionInfo],
) -> Result<u8, DispatchError> {
    ble_get_connection_state_impl(service, info, proto::BLE_GET_CONNECTION_STATE_LEGACY)
}

/// BleGetGattClientConditionList (cmd 39, 5.1.0+).
pub(crate) fn ble_get_gatt_client_condition_list(
    service: &Session,
    list: &mut BtmGattClientConditionList,
) -> Result<(), DispatchError> {
    dispatch_out_buf_ptr_fixed(service, list, proto::BLE_GET_GATT_CLIENT_CONDITION_LIST)
}

/// BleGetGattClientConditionList \[5.0.0-5.0.2\] (cmd 27).
pub(crate) fn ble_get_gatt_client_condition_list_legacy(
    service: &Session,
    list: &mut BtmGattClientConditionList,
) -> Result<(), DispatchError> {
    dispatch_out_buf_ptr_fixed(
        service,
        list,
        proto::BLE_GET_GATT_CLIENT_CONDITION_LIST_LEGACY,
    )
}

// ---------------------------------------------------------------------------
// BLE pairing commands (5.0.0+/5.1.0+)
// ---------------------------------------------------------------------------

/// AcquireBlePairingEvent (cmd 40, 5.1.0+).
pub(crate) fn acquire_ble_pairing_event(
    service: &Session,
) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_BLE_PAIRING_EVENT)
}

/// AcquireBlePairingEvent \[5.0.0-5.0.2\] (cmd 28).
pub(crate) fn acquire_ble_pairing_event_legacy(
    service: &Session,
) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_BLE_PAIRING_EVENT_LEGACY)
}

/// BlePairDevice (cmd 41, 5.1.0+).
pub(crate) fn ble_pair_device(
    service: &Session,
    connection_handle: u32,
    param: &BtdrvBleAdvertisePacketParameter,
) -> Result<(), DispatchError> {
    let input = BlePairDeviceIn {
        param: *param,
        connection_handle,
    };
    dispatch_in(service, proto::BLE_PAIR_DEVICE, input)
}

/// BleUnpairDeviceOnBoth (cmd 42, 5.1.0+).
pub(crate) fn ble_unpair_device_on_both(
    service: &Session,
    connection_handle: u32,
    param: &BtdrvBleAdvertisePacketParameter,
) -> Result<(), DispatchError> {
    let input = BlePairDeviceIn {
        param: *param,
        connection_handle,
    };
    dispatch_in(service, proto::BLE_UNPAIR_DEVICE_ON_BOTH, input)
}

/// BleUnPairDevice (cmd 43, 5.1.0+).
pub(crate) fn ble_unpair_device(
    service: &Session,
    addr: &BtdrvAddress,
    param: &BtdrvBleAdvertisePacketParameter,
) -> Result<(), DispatchError> {
    let input = BleUnpairDeviceIn {
        addr: *addr,
        param: *param,
    };
    dispatch_in(service, proto::BLE_UNPAIR_DEVICE, input)
}

/// BleGetPairedAddresses (cmd 44, 5.1.0+).
pub(crate) fn ble_get_paired_addresses(
    service: &Session,
    param: &BtdrvBleAdvertisePacketParameter,
    addrs: &mut [BtdrvAddress],
) -> Result<u8, DispatchError> {
    // SAFETY: `param` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const *param).cast::<u8>(),
            size_of::<BtdrvBleAdvertisePacketParameter>(),
        )
    };
    // SAFETY: `addrs` is a valid `&mut [BtdrvAddress]`; viewing it as a byte
    // slice for the OUT buffer is sound, and the byte slice borrows `addrs`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            addrs.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(addrs),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::BLE_GET_PAIRED_ADDRESSES)
        .in_raw(in_bytes)
        .out_size(size_of::<u8>())
        .out_buffer(out_bytes, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least 1 byte.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) })
}

// ---------------------------------------------------------------------------
// GATT service discovery commands (5.0.0+/5.1.0+)
// ---------------------------------------------------------------------------

/// AcquireBleServiceDiscoveryEvent (cmd 45, 5.1.0+).
pub(crate) fn acquire_ble_service_discovery_event(
    service: &Session,
) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_BLE_SERVICE_DISCOVERY_EVENT)
}

/// GetGattServices (cmd 46, 5.1.0+).
pub(crate) fn get_gatt_services(
    service: &Session,
    connection_handle: u32,
    services: &mut [BtmGattService],
) -> Result<u8, DispatchError> {
    get_gatt_services_impl(
        service,
        connection_handle,
        services,
        proto::GET_GATT_SERVICES,
    )
}

/// GetGattServices \[5.0.0-5.0.2\] (cmd 29).
pub(crate) fn get_gatt_services_legacy(
    service: &Session,
    connection_handle: u32,
    services: &mut [BtmGattService],
) -> Result<u8, DispatchError> {
    get_gatt_services_impl(
        service,
        connection_handle,
        services,
        proto::GET_GATT_SERVICES_LEGACY,
    )
}

/// GetGattService (cmd 47, 5.1.0+).
pub(crate) fn get_gatt_service(
    service: &Session,
    connection_handle: u32,
    uuid: &BtdrvGattAttributeUuid,
    out_service: &mut BtmGattService,
) -> Result<bool, DispatchError> {
    get_gatt_service_impl(
        service,
        connection_handle,
        uuid,
        out_service,
        proto::GET_GATT_SERVICE,
    )
}

/// GetGattService \[5.0.0-5.0.2\] (cmd 30).
pub(crate) fn get_gatt_service_legacy(
    service: &Session,
    connection_handle: u32,
    uuid: &BtdrvGattAttributeUuid,
    out_service: &mut BtmGattService,
) -> Result<bool, DispatchError> {
    get_gatt_service_impl(
        service,
        connection_handle,
        uuid,
        out_service,
        proto::GET_GATT_SERVICE_LEGACY,
    )
}

/// GetGattIncludedServices (cmd 48, 5.1.0+).
pub(crate) fn get_gatt_included_services(
    service: &Session,
    connection_handle: u32,
    service_handle: u16,
    services: &mut [BtmGattService],
) -> Result<u8, DispatchError> {
    get_gatt_service_data(
        service,
        connection_handle,
        service_handle,
        services,
        proto::GET_GATT_INCLUDED_SERVICES,
    )
}

/// GetGattIncludedServices \[5.0.0-5.0.2\] (cmd 31).
pub(crate) fn get_gatt_included_services_legacy(
    service: &Session,
    connection_handle: u32,
    service_handle: u16,
    services: &mut [BtmGattService],
) -> Result<u8, DispatchError> {
    get_gatt_service_data(
        service,
        connection_handle,
        service_handle,
        services,
        proto::GET_GATT_INCLUDED_SERVICES_LEGACY,
    )
}

/// GetBelongingService (cmd 49, 5.1.0+).
pub(crate) fn get_belonging_service(
    service: &Session,
    connection_handle: u32,
    attribute_handle: u16,
    out_service: &mut BtmGattService,
) -> Result<bool, DispatchError> {
    get_belonging_service_impl(
        service,
        connection_handle,
        attribute_handle,
        out_service,
        proto::GET_BELONGING_SERVICE,
    )
}

/// GetBelongingService \[5.0.0-5.0.2\] (cmd 32).
pub(crate) fn get_belonging_service_legacy(
    service: &Session,
    connection_handle: u32,
    attribute_handle: u16,
    out_service: &mut BtmGattService,
) -> Result<bool, DispatchError> {
    get_belonging_service_impl(
        service,
        connection_handle,
        attribute_handle,
        out_service,
        proto::GET_BELONGING_SERVICE_LEGACY,
    )
}

/// GetGattCharacteristics (cmd 50, 5.1.0+).
pub(crate) fn get_gatt_characteristics(
    service: &Session,
    connection_handle: u32,
    service_handle: u16,
    characteristics: &mut [BtmGattCharacteristic],
) -> Result<u8, DispatchError> {
    get_gatt_service_data(
        service,
        connection_handle,
        service_handle,
        characteristics,
        proto::GET_GATT_CHARACTERISTICS,
    )
}

/// GetGattCharacteristics \[5.0.0-5.0.2\] (cmd 33).
pub(crate) fn get_gatt_characteristics_legacy(
    service: &Session,
    connection_handle: u32,
    service_handle: u16,
    characteristics: &mut [BtmGattCharacteristic],
) -> Result<u8, DispatchError> {
    get_gatt_service_data(
        service,
        connection_handle,
        service_handle,
        characteristics,
        proto::GET_GATT_CHARACTERISTICS_LEGACY,
    )
}

/// GetGattDescriptors (cmd 51, 5.1.0+).
pub(crate) fn get_gatt_descriptors(
    service: &Session,
    connection_handle: u32,
    char_handle: u16,
    descriptors: &mut [BtmGattDescriptor],
) -> Result<u8, DispatchError> {
    get_gatt_service_data(
        service,
        connection_handle,
        char_handle,
        descriptors,
        proto::GET_GATT_DESCRIPTORS,
    )
}

/// GetGattDescriptors \[5.0.0-5.0.2\] (cmd 34).
pub(crate) fn get_gatt_descriptors_legacy(
    service: &Session,
    connection_handle: u32,
    char_handle: u16,
    descriptors: &mut [BtmGattDescriptor],
) -> Result<u8, DispatchError> {
    get_gatt_service_data(
        service,
        connection_handle,
        char_handle,
        descriptors,
        proto::GET_GATT_DESCRIPTORS_LEGACY,
    )
}

// ---------------------------------------------------------------------------
// BLE MTU commands (5.0.0+/5.1.0+)
// ---------------------------------------------------------------------------

/// AcquireBleMtuConfigEvent (cmd 52, 5.1.0+).
pub(crate) fn acquire_ble_mtu_config_event(
    service: &Session,
) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_BLE_MTU_CONFIG_EVENT)
}

/// AcquireBleMtuConfigEvent \[5.0.0-5.0.2\] (cmd 35).
pub(crate) fn acquire_ble_mtu_config_event_legacy(
    service: &Session,
) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_BLE_MTU_CONFIG_EVENT_LEGACY)
}

/// ConfigureBleMtu (cmd 53, 5.1.0+).
pub(crate) fn configure_ble_mtu(
    service: &Session,
    connection_handle: u32,
    mtu: u16,
) -> Result<(), DispatchError> {
    let input = ConfigureBleMtuIn {
        mtu,
        pad: 0,
        connection_handle,
    };
    dispatch_in(service, proto::CONFIGURE_BLE_MTU, input)
}

/// ConfigureBleMtu \[5.0.0-5.0.2\] (cmd 36).
pub(crate) fn configure_ble_mtu_legacy(
    service: &Session,
    connection_handle: u32,
    mtu: u16,
) -> Result<(), DispatchError> {
    let input = ConfigureBleMtuIn {
        mtu,
        pad: 0,
        connection_handle,
    };
    dispatch_in(service, proto::CONFIGURE_BLE_MTU_LEGACY, input)
}

/// GetBleMtu (cmd 54, 5.1.0+).
pub(crate) fn get_ble_mtu(service: &Session, connection_handle: u32) -> Result<u16, DispatchError> {
    dispatch_in_out(service, proto::GET_BLE_MTU, connection_handle)
}

/// GetBleMtu \[5.0.0-5.0.2\] (cmd 37).
pub(crate) fn get_ble_mtu_legacy(
    service: &Session,
    connection_handle: u32,
) -> Result<u16, DispatchError> {
    dispatch_in_out(service, proto::GET_BLE_MTU_LEGACY, connection_handle)
}

// ---------------------------------------------------------------------------
// GATT data path commands (5.0.0+/5.1.0+)
// ---------------------------------------------------------------------------

/// RegisterBleGattDataPath (cmd 55, 5.1.0+).
pub(crate) fn register_ble_gatt_data_path(
    service: &Session,
    path: &BtmBleDataPath,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::REGISTER_BLE_GATT_DATA_PATH, *path)
}

/// RegisterBleGattDataPath \[5.0.0-5.0.2\] (cmd 38).
pub(crate) fn register_ble_gatt_data_path_legacy(
    service: &Session,
    path: &BtmBleDataPath,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::REGISTER_BLE_GATT_DATA_PATH_LEGACY, *path)
}

/// UnregisterBleGattDataPath (cmd 56, 5.1.0+).
pub(crate) fn unregister_ble_gatt_data_path(
    service: &Session,
    path: &BtmBleDataPath,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::UNREGISTER_BLE_GATT_DATA_PATH, *path)
}

/// UnregisterBleGattDataPath \[5.0.0-5.0.2\] (cmd 39).
pub(crate) fn unregister_ble_gatt_data_path_legacy(
    service: &Session,
    path: &BtmBleDataPath,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::UNREGISTER_BLE_GATT_DATA_PATH_LEGACY, *path)
}

// ---------------------------------------------------------------------------
// Applet resource user ID commands (5.0.0+/5.1.0+)
// ---------------------------------------------------------------------------

/// RegisterAppletResourceUserId (cmd 57, 5.1.0+).
pub(crate) fn register_applet_resource_user_id(
    service: &Session,
    applet_resource_user_id: u64,
    unk: u32,
) -> Result<(), DispatchError> {
    let input = RegisterAruidIn {
        unk,
        applet_resource_user_id,
    };
    dispatch_in(service, proto::REGISTER_APPLET_RESOURCE_USER_ID, input)
}

/// RegisterAppletResourceUserId \[5.0.0-5.0.2\] (cmd 40).
pub(crate) fn register_applet_resource_user_id_legacy(
    service: &Session,
    applet_resource_user_id: u64,
    unk: u32,
) -> Result<(), DispatchError> {
    let input = RegisterAruidIn {
        unk,
        applet_resource_user_id,
    };
    dispatch_in(
        service,
        proto::REGISTER_APPLET_RESOURCE_USER_ID_LEGACY,
        input,
    )
}

/// UnregisterAppletResourceUserId (cmd 58, 5.1.0+).
pub(crate) fn unregister_applet_resource_user_id(
    service: &Session,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::UNREGISTER_APPLET_RESOURCE_USER_ID,
        applet_resource_user_id,
    )
}

/// UnregisterAppletResourceUserId \[5.0.0-5.0.2\] (cmd 41).
pub(crate) fn unregister_applet_resource_user_id_legacy(
    service: &Session,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::UNREGISTER_APPLET_RESOURCE_USER_ID_LEGACY,
        applet_resource_user_id,
    )
}

/// SetAppletResourceUserId (cmd 59, 5.1.0+).
pub(crate) fn set_applet_resource_user_id(
    service: &Session,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::SET_APPLET_RESOURCE_USER_ID,
        applet_resource_user_id,
    )
}

/// SetAppletResourceUserId \[5.0.0-5.0.2\] (cmd 42).
pub(crate) fn set_applet_resource_user_id_legacy(
    service: &Session,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::SET_APPLET_RESOURCE_USER_ID_LEGACY,
        applet_resource_user_id,
    )
}

// ---------------------------------------------------------------------------
// Shared dispatch helpers
// ---------------------------------------------------------------------------

/// Dispatches a command with a HipcPointer fixed-size input buffer.
fn dispatch_in_buf_ptr_fixed<T>(
    service: &Session,
    buf: &T,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `buf` is a valid reference; viewing its bytes as a slice for
    // the IN buffer is sound, and the byte slice borrows `buf`.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((buf as *const T).cast::<u8>(), size_of::<T>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(cmd_id)
        .in_buffer(
            in_bytes,
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Dispatches a command with a HipcPointer fixed-size output buffer.
fn dispatch_out_buf_ptr_fixed<T>(
    service: &Session,
    buf: &mut T,
    cmd_id: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `buf` is a valid mutable reference; viewing its bytes as a slice
    // for the OUT buffer is sound, and the byte slice borrows `buf`.
    let out_bytes =
        unsafe { core::slice::from_raw_parts_mut((buf as *mut T).cast::<u8>(), size_of::<T>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(cmd_id)
        .out_buffer(
            out_bytes,
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Dispatches a command that returns BLE scan results via HipcMapAlias output.
fn get_ble_scan_results(
    service: &Session,
    results: &mut [BtdrvBleScanResult],
    cmd_id: u32,
) -> Result<u8, DispatchError> {
    // SAFETY: `results` is a valid `&mut [BtdrvBleScanResult]`; viewing it as
    // a byte slice for the OUT buffer is sound, and the byte slice borrows `results`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            results.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(results),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<u8>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least 1 byte.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) })
}

/// Dispatches BleGetConnectionState with a HipcPointer output buffer.
fn ble_get_connection_state_impl(
    service: &Session,
    info: &mut [BtdrvBleConnectionInfo],
    cmd_id: u32,
) -> Result<u8, DispatchError> {
    // SAFETY: `info` is a valid `&mut [BtdrvBleConnectionInfo]`; viewing it as
    // a byte slice for the OUT buffer is sound, and the byte slice borrows `info`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            info.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(info),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<u8>())
        .out_buffer(out_bytes, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least 1 byte.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) })
}

/// Dispatches GetGattServices (in u32 + HipcMapAlias out).
fn get_gatt_services_impl(
    service: &Session,
    connection_handle: u32,
    services: &mut [BtmGattService],
    cmd_id: u32,
) -> Result<u8, DispatchError> {
    // SAFETY: `connection_handle` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const connection_handle).cast::<u8>(),
            size_of::<u32>(),
        )
    };
    // SAFETY: `services` is a valid `&mut [BtmGattService]`; viewing it as a
    // byte slice for the OUT buffer is sound, and the byte slice borrows `services`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            services.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(services),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<u8>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least 1 byte.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) })
}

/// Dispatches GetGattService (in connection_handle+uuid, out flag + pointer fixed service).
fn get_gatt_service_impl(
    service: &Session,
    connection_handle: u32,
    uuid: &BtdrvGattAttributeUuid,
    out_service: &mut BtmGattService,
    cmd_id: u32,
) -> Result<bool, DispatchError> {
    let input = GetGattServiceIn {
        connection_handle,
        uuid: *uuid,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<GetGattServiceIn>(),
        )
    };
    // SAFETY: `out_service` is a valid mutable reference; viewing its bytes as
    // a slice for the OUT buffer is sound, and the byte slice borrows `out_service`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            (out_service as *mut BtmGattService).cast::<u8>(),
            size_of::<BtmGattService>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<u8>())
        .out_buffer(
            out_bytes,
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least 1 byte.
    let flag: u8 = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    Ok(flag & 1 != 0)
}

/// Dispatches GATT service data query (in handle+connection_handle, HipcMapAlias out).
fn get_gatt_service_data<T>(
    service: &Session,
    connection_handle: u32,
    handle: u16,
    buffer: &mut [T],
    cmd_id: u32,
) -> Result<u8, DispatchError> {
    let input = HandleConnectionIn {
        handle,
        pad: 0,
        connection_handle,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<HandleConnectionIn>(),
        )
    };
    // SAFETY: `buffer` is a valid mutable slice; viewing it as a byte slice for
    // the OUT buffer is sound, and the byte slice borrows `buffer`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            buffer.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(buffer),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<u8>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least 1 byte.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) })
}

/// Dispatches GetBelongingService (in handle+connection_handle, pointer fixed out + flag).
fn get_belonging_service_impl(
    service: &Session,
    connection_handle: u32,
    attribute_handle: u16,
    out_service: &mut BtmGattService,
    cmd_id: u32,
) -> Result<bool, DispatchError> {
    let input = HandleConnectionIn {
        handle: attribute_handle,
        pad: 0,
        connection_handle,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<HandleConnectionIn>(),
        )
    };
    // SAFETY: `out_service` is a valid mutable reference; viewing its bytes as
    // a slice for the OUT buffer is sound, and the byte slice borrows `out_service`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            (out_service as *mut BtmGattService).cast::<u8>(),
            size_of::<BtmGattService>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<u8>())
        .out_buffer(
            out_bytes,
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least 1 byte.
    let flag: u8 = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    Ok(flag & 1 != 0)
}

/// Dispatches a command that returns a copy handle for an event.
fn acquire_event(service: &Session, cmd_id: u32) -> Result<u32, AcquireEventError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

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

/// Dispatches a command that returns a copy handle plus an out flag that must be nonzero.
fn acquire_event_with_flag(
    service: &Session,
    cmd_id: u32,
) -> Result<u32, AcquireEventWithFlagError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

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

/// Error returned by event acquisition commands (copy handle only, pre-3.0.0).
#[derive(Debug, thiserror::Error)]
pub enum AcquireEventError {
    #[error("failed to dispatch event acquisition")]
    Dispatch(#[source] DispatchError),
    #[error("event acquisition response did not include expected copy handle")]
    MissingHandle,
}
