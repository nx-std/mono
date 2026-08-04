//! CMIF protocol operations for the Bluetooth Manager User service.

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
        dispatch_in_pid,
        dispatch_no_io,
    },
    proto,
    types::{
        BleConnectIn,
        BtdrvAddress,
        BtdrvBleAdvertisePacketParameter,
        BtdrvBleConnectionInfo,
        BtdrvBleScanResult,
        BtdrvGattAttributeUuid,
        BtmBleDataPath,
        BtmGattCharacteristic,
        BtmGattDescriptor,
        BtmGattService,
        ConfigureBleMtuIn,
        GattDataPathAruidIn,
        GattServiceDataIn,
        GetBleMtuIn,
        GetGattServiceIn,
        GetGattServicesIn,
        PairDeviceIn,
        ScanParamAruidIn,
        ScanUuidAruidIn,
        UnpairDevice2In,
    },
};

// ---------------------------------------------------------------------------
// Root service commands
// ---------------------------------------------------------------------------

/// Gets the IBtmUserCore sub-object (cmd 0).
pub(crate) fn get_core(service: &Session) -> Result<u32, GetCoreError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::GET_CORE)
        .send(&mut ipc_buf)
        .map_err(GetCoreError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(GetCoreError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

// ---------------------------------------------------------------------------
// BLE scan commands
// ---------------------------------------------------------------------------

/// AcquireBleScanEvent (cmd 0).
pub(crate) fn acquire_ble_scan_event(service: &Session) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_BLE_SCAN_EVENT)
}

/// GetBleScanFilterParameter (cmd 1).
pub(crate) fn get_ble_scan_filter_parameter(
    service: &Session,
    parameter_id: u16,
) -> Result<BtdrvBleAdvertisePacketParameter, DispatchError> {
    dispatch_in_out(service, proto::GET_BLE_SCAN_FILTER_PARAMETER, &parameter_id)
}

/// GetBleScanFilterParameter2 (cmd 2).
pub(crate) fn get_ble_scan_filter_parameter2(
    service: &Session,
    parameter_id: u16,
) -> Result<BtdrvGattAttributeUuid, DispatchError> {
    dispatch_in_out(
        service,
        proto::GET_BLE_SCAN_FILTER_PARAMETER2,
        &parameter_id,
    )
}

/// StartBleScanForGeneral (cmd 3).
pub(crate) fn start_ble_scan_for_general(
    service: &Session,
    param: &BtdrvBleAdvertisePacketParameter,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = ScanParamAruidIn {
        param: *param,
        applet_resource_user_id,
    };
    dispatch_in_pid(service, proto::START_BLE_SCAN_FOR_GENERAL, &input)
}

/// StopBleScanForGeneral (cmd 4).
pub(crate) fn stop_ble_scan_for_general(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::STOP_BLE_SCAN_FOR_GENERAL)
}

/// GetBleScanResultsForGeneral (cmd 5).
pub(crate) fn get_ble_scan_results_for_general(
    service: &Session,
    results: &mut [BtdrvBleScanResult],
    applet_resource_user_id: u64,
) -> Result<u8, DispatchError> {
    get_ble_scan_results(
        service,
        results,
        applet_resource_user_id,
        proto::GET_BLE_SCAN_RESULTS_FOR_GENERAL,
    )
}

/// StartBleScanForPaired (cmd 6).
pub(crate) fn start_ble_scan_for_paired(
    service: &Session,
    param: &BtdrvBleAdvertisePacketParameter,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = ScanParamAruidIn {
        param: *param,
        applet_resource_user_id,
    };
    dispatch_in_pid(service, proto::START_BLE_SCAN_FOR_PAIRED, &input)
}

/// StopBleScanForPaired (cmd 7).
pub(crate) fn stop_ble_scan_for_paired(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::STOP_BLE_SCAN_FOR_PAIRED)
}

/// StartBleScanForSmartDevice (cmd 8).
pub(crate) fn start_ble_scan_for_smart_device(
    service: &Session,
    uuid: &BtdrvGattAttributeUuid,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = ScanUuidAruidIn {
        uuid: *uuid,
        pad: 0,
        applet_resource_user_id,
    };
    dispatch_in_pid(service, proto::START_BLE_SCAN_FOR_SMART_DEVICE, &input)
}

/// StopBleScanForSmartDevice (cmd 9).
pub(crate) fn stop_ble_scan_for_smart_device(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::STOP_BLE_SCAN_FOR_SMART_DEVICE)
}

/// GetBleScanResultsForSmartDevice (cmd 10).
pub(crate) fn get_ble_scan_results_for_smart_device(
    service: &Session,
    results: &mut [BtdrvBleScanResult],
    applet_resource_user_id: u64,
) -> Result<u8, DispatchError> {
    get_ble_scan_results(
        service,
        results,
        applet_resource_user_id,
        proto::GET_BLE_SCAN_RESULTS_FOR_SMART_DEVICE,
    )
}

// ---------------------------------------------------------------------------
// BLE connection commands
// ---------------------------------------------------------------------------

/// AcquireBleConnectionEvent (cmd 17).
pub(crate) fn acquire_ble_connection_event(
    service: &Session,
) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_BLE_CONNECTION_EVENT)
}

/// BleConnect (cmd 18).
pub(crate) fn ble_connect(
    service: &Session,
    addr: &BtdrvAddress,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = BleConnectIn {
        addr: *addr,
        pad: [0; 2],
        applet_resource_user_id,
    };
    dispatch_in_pid(service, proto::BLE_CONNECT, &input)
}

/// BleDisconnect (cmd 19).
pub(crate) fn ble_disconnect(
    service: &Session,
    connection_handle: u32,
) -> Result<(), DispatchError> {
    dispatch_in(service, proto::BLE_DISCONNECT, &connection_handle)
}

/// BleGetConnectionState (cmd 20).
pub(crate) fn ble_get_connection_state(
    service: &Session,
    info: &mut [BtdrvBleConnectionInfo],
    applet_resource_user_id: u64,
) -> Result<u8, DispatchError> {
    // SAFETY: `applet_resource_user_id` is a `Copy` value on the stack, valid
    // until `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const applet_resource_user_id).cast::<u8>(),
            size_of::<u64>(),
        )
    };
    // SAFETY: `info` is a valid `&mut` slice; viewing it as bytes for the
    // OUT buffer is sound, and the byte slice borrows `info`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            info.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(info),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::BLE_GET_CONNECTION_STATE)
        .in_raw(in_bytes)
        .out_size(size_of::<u8>())
        .out_buffer(out_bytes, BufferAttr::HIPC_POINTER)
        .send_pid()
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least 1 byte.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) })
}

// ---------------------------------------------------------------------------
// BLE pairing commands
// ---------------------------------------------------------------------------

/// AcquireBlePairingEvent (cmd 21).
pub(crate) fn acquire_ble_pairing_event(
    service: &Session,
) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_BLE_PAIRING_EVENT)
}

/// BlePairDevice (cmd 22).
pub(crate) fn ble_pair_device(
    service: &Session,
    connection_handle: u32,
    param: &BtdrvBleAdvertisePacketParameter,
) -> Result<(), DispatchError> {
    let input = PairDeviceIn {
        param: *param,
        connection_handle,
    };
    dispatch_in(service, proto::BLE_PAIR_DEVICE, &input)
}

/// BleUnPairDevice (cmd 23).
pub(crate) fn ble_unpair_device(
    service: &Session,
    connection_handle: u32,
    param: &BtdrvBleAdvertisePacketParameter,
) -> Result<(), DispatchError> {
    let input = PairDeviceIn {
        param: *param,
        connection_handle,
    };
    dispatch_in(service, proto::BLE_UNPAIR_DEVICE, &input)
}

/// BleUnPairDevice2 (cmd 24).
pub(crate) fn ble_unpair_device2(
    service: &Session,
    addr: &BtdrvAddress,
    param: &BtdrvBleAdvertisePacketParameter,
) -> Result<(), DispatchError> {
    let input = UnpairDevice2In {
        addr: *addr,
        param: *param,
    };
    dispatch_in(service, proto::BLE_UNPAIR_DEVICE2, &input)
}

/// BleGetPairedDevices (cmd 25).
pub(crate) fn ble_get_paired_devices(
    service: &Session,
    param: &BtdrvBleAdvertisePacketParameter,
    addrs: &mut [BtdrvAddress],
) -> Result<u8, DispatchError> {
    // SAFETY: `param` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (param as *const BtdrvBleAdvertisePacketParameter).cast::<u8>(),
            size_of::<BtdrvBleAdvertisePacketParameter>(),
        )
    };
    // SAFETY: `addrs` is a valid `&mut` slice; viewing it as bytes for the
    // OUT buffer is sound, and the byte slice borrows `addrs`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            addrs.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(addrs),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::BLE_GET_PAIRED_DEVICES)
        .in_raw(in_bytes)
        .out_size(size_of::<u8>())
        .out_buffer(out_bytes, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least 1 byte.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) })
}

// ---------------------------------------------------------------------------
// GATT service discovery commands
// ---------------------------------------------------------------------------

/// AcquireBleServiceDiscoveryEvent (cmd 26).
pub(crate) fn acquire_ble_service_discovery_event(
    service: &Session,
) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_BLE_SERVICE_DISCOVERY_EVENT)
}

/// GetGattServices (cmd 27).
pub(crate) fn get_gatt_services(
    service: &Session,
    connection_handle: u32,
    services: &mut [BtmGattService],
    applet_resource_user_id: u64,
) -> Result<u8, DispatchError> {
    let input = GetGattServicesIn {
        connection_handle,
        pad: 0,
        applet_resource_user_id,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<GetGattServicesIn>(),
        )
    };
    // SAFETY: `services` is a valid `&mut` slice; viewing it as bytes for the
    // OUT buffer is sound, and the byte slice borrows `services`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            services.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(services),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::GET_GATT_SERVICES)
        .in_raw(in_bytes)
        .out_size(size_of::<u8>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send_pid()
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least 1 byte.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) })
}

/// GetGattService (cmd 28).
pub(crate) fn get_gatt_service(
    service: &Session,
    connection_handle: u32,
    uuid: &BtdrvGattAttributeUuid,
    out_service: &mut BtmGattService,
    applet_resource_user_id: u64,
) -> Result<bool, DispatchError> {
    let input = GetGattServiceIn {
        connection_handle,
        uuid: *uuid,
        applet_resource_user_id,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<GetGattServiceIn>(),
        )
    };
    // SAFETY: `out_service` is a valid exclusive reference; viewing it as
    // bytes for the OUT buffer is sound, and the byte slice borrows it.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            (out_service as *mut BtmGattService).cast::<u8>(),
            size_of::<BtmGattService>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::GET_GATT_SERVICE)
        .in_raw(in_bytes)
        .out_size(size_of::<u8>())
        .out_buffer(
            out_bytes,
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send_pid()
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least 1 byte.
    let flag: u8 = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    Ok(flag & 1 != 0)
}

/// GetGattIncludedServices (cmd 29).
pub(crate) fn get_gatt_included_services(
    service: &Session,
    connection_handle: u32,
    service_handle: u16,
    services: &mut [BtmGattService],
    applet_resource_user_id: u64,
) -> Result<u8, DispatchError> {
    get_gatt_service_data(
        service,
        connection_handle,
        service_handle,
        services.as_mut_ptr().cast::<u8>(),
        core::mem::size_of_val(services),
        applet_resource_user_id,
        proto::GET_GATT_INCLUDED_SERVICES,
    )
}

/// GetBelongingGattService (cmd 30).
pub(crate) fn get_belonging_gatt_service(
    service: &Session,
    connection_handle: u32,
    attribute_handle: u16,
    out_service: &mut BtmGattService,
    applet_resource_user_id: u64,
) -> Result<bool, DispatchError> {
    let input = GattServiceDataIn {
        handle: attribute_handle,
        pad: 0,
        connection_handle,
        applet_resource_user_id,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<GattServiceDataIn>(),
        )
    };
    // SAFETY: `out_service` is a valid exclusive reference; viewing it as
    // bytes for the OUT buffer is sound, and the byte slice borrows it.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            (out_service as *mut BtmGattService).cast::<u8>(),
            size_of::<BtmGattService>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::GET_BELONGING_GATT_SERVICE)
        .in_raw(in_bytes)
        .out_size(size_of::<u8>())
        .out_buffer(
            out_bytes,
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send_pid()
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least 1 byte.
    let flag: u8 = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    Ok(flag & 1 != 0)
}

/// GetGattCharacteristics (cmd 31).
pub(crate) fn get_gatt_characteristics(
    service: &Session,
    connection_handle: u32,
    service_handle: u16,
    characteristics: &mut [BtmGattCharacteristic],
    applet_resource_user_id: u64,
) -> Result<u8, DispatchError> {
    get_gatt_service_data(
        service,
        connection_handle,
        service_handle,
        characteristics.as_mut_ptr().cast::<u8>(),
        core::mem::size_of_val(characteristics),
        applet_resource_user_id,
        proto::GET_GATT_CHARACTERISTICS,
    )
}

/// GetGattDescriptors (cmd 32).
pub(crate) fn get_gatt_descriptors(
    service: &Session,
    connection_handle: u32,
    char_handle: u16,
    descriptors: &mut [BtmGattDescriptor],
    applet_resource_user_id: u64,
) -> Result<u8, DispatchError> {
    get_gatt_service_data(
        service,
        connection_handle,
        char_handle,
        descriptors.as_mut_ptr().cast::<u8>(),
        core::mem::size_of_val(descriptors),
        applet_resource_user_id,
        proto::GET_GATT_DESCRIPTORS,
    )
}

// ---------------------------------------------------------------------------
// BLE MTU commands
// ---------------------------------------------------------------------------

/// AcquireBleMtuConfigEvent (cmd 33).
pub(crate) fn acquire_ble_mtu_config_event(
    service: &Session,
) -> Result<u32, AcquireEventWithFlagError> {
    acquire_event_with_flag(service, proto::ACQUIRE_BLE_MTU_CONFIG_EVENT)
}

/// ConfigureBleMtu (cmd 34).
pub(crate) fn configure_ble_mtu(
    service: &Session,
    connection_handle: u32,
    mtu: u16,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = ConfigureBleMtuIn {
        mtu,
        pad: 0,
        connection_handle,
        applet_resource_user_id,
    };
    dispatch_in_pid(service, proto::CONFIGURE_BLE_MTU, &input)
}

/// GetBleMtu (cmd 35).
pub(crate) fn get_ble_mtu(
    service: &Session,
    connection_handle: u32,
    applet_resource_user_id: u64,
) -> Result<u16, DispatchError> {
    let input = GetBleMtuIn {
        connection_handle,
        pad: 0,
        applet_resource_user_id,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<GetBleMtuIn>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::GET_BLE_MTU)
        .in_raw(in_bytes)
        .out_size(size_of::<u16>())
        .send_pid()
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<u16>() bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u16>()) })
}

// ---------------------------------------------------------------------------
// GATT data path commands
// ---------------------------------------------------------------------------

/// RegisterBleGattDataPath (cmd 36).
pub(crate) fn register_ble_gatt_data_path(
    service: &Session,
    path: &BtmBleDataPath,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = GattDataPathAruidIn {
        path: *path,
        applet_resource_user_id,
    };
    dispatch_in_pid(service, proto::REGISTER_BLE_GATT_DATA_PATH, &input)
}

/// UnregisterBleGattDataPath (cmd 37).
pub(crate) fn unregister_ble_gatt_data_path(
    service: &Session,
    path: &BtmBleDataPath,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = GattDataPathAruidIn {
        path: *path,
        applet_resource_user_id,
    };
    dispatch_in_pid(service, proto::UNREGISTER_BLE_GATT_DATA_PATH, &input)
}

// ---------------------------------------------------------------------------
// Shared dispatch helpers
// ---------------------------------------------------------------------------

/// Dispatches a command that returns BLE scan results via HipcMapAlias output
/// buffer, sending PID and an applet resource user ID.
fn get_ble_scan_results(
    service: &Session,
    results: &mut [BtdrvBleScanResult],
    applet_resource_user_id: u64,
    cmd_id: u32,
) -> Result<u8, DispatchError> {
    // SAFETY: `applet_resource_user_id` is a `Copy` value on the stack, valid
    // until `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const applet_resource_user_id).cast::<u8>(),
            size_of::<u64>(),
        )
    };
    // SAFETY: `results` is a valid `&mut` slice; viewing it as bytes for the
    // OUT buffer is sound, and the byte slice borrows `results`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            results.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(results),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<u8>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send_pid()
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least 1 byte.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) })
}

/// Dispatches a GATT service data query (cmds 29, 31, 32) with a handle,
/// connection handle, PID, and ARUID, returning count via HipcMapAlias buffer.
fn get_gatt_service_data(
    service: &Session,
    connection_handle: u32,
    handle: u16,
    buffer: *mut u8,
    buffer_size: usize,
    applet_resource_user_id: u64,
    cmd_id: u32,
) -> Result<u8, DispatchError> {
    let input = GattServiceDataIn {
        handle,
        pad: 0,
        connection_handle,
        applet_resource_user_id,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<GattServiceDataIn>(),
        )
    };
    // SAFETY: `buffer` is a valid pointer to `buffer_size` writable bytes,
    // exclusively borrowed for the duration of this call.
    let out_bytes = unsafe { core::slice::from_raw_parts_mut(buffer, buffer_size) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<u8>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send_pid()
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least 1 byte.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) })
}

/// Dispatches a command that returns a copy handle for an event plus an out
/// flag byte that must be nonzero (libnx ShouldNotHappen check).
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

    if result.copy_handles.is_empty() {
        return Err(AcquireEventWithFlagError::MissingHandle);
    }

    Ok(result.copy_handles[0])
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
