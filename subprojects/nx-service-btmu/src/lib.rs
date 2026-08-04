//! Bluetooth Manager User service (`btm:u`) implementation.
//!
//! Provides BLE scanning, connection management, GATT service discovery,
//! pairing, MTU configuration, and data path registration for the
//! Nintendo Switch. Only available on \[5.0.0+\].
//!
//! ## Usage
//!
//! 1. Connect to the service via [`connect_cmif`].
//! 2. Call BLE scan, connection, GATT, pairing, or MTU methods on [`BtmuService`].
//! 3. The session is closed automatically on `Drop`.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{
    BorrowedSessionHandle,
    OwnedSessionHandle,
    Session,
};
use nx_svc::ipc::Handle;

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::AcquireEventWithFlagError,
    proto::SERVICE_NAME,
    types::{
        BtdrvAddress,
        BtdrvBleAdvertisePacketParameter,
        BtdrvBleConnectionInfo,
        BtdrvBleScanResult,
        BtdrvGattAttributeUuid,
        BtmBleDataPath,
        BtmGattCharacteristic,
        BtmGattDescriptor,
        BtmGattService,
    },
};

/// Bluetooth Manager User service wrapper (IBtmUserCore).
pub struct BtmuService(Session);

impl BtmuService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

// ---------------------------------------------------------------------------
// BLE scan commands
// ---------------------------------------------------------------------------

impl BtmuService {
    /// Acquires the BLE scan event (cmd 0).
    ///
    /// Returns a copy handle for the event (autoclear=true).
    #[inline]
    pub fn acquire_ble_scan_event(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_ble_scan_event(&self.0)
    }

    /// Gets the BLE scan filter parameter (cmd 1).
    #[inline]
    pub fn get_ble_scan_filter_parameter(
        &self,
        parameter_id: u16,
    ) -> Result<BtdrvBleAdvertisePacketParameter, nx_sf::service::DispatchError> {
        cmif::get_ble_scan_filter_parameter(&self.0, parameter_id)
    }

    /// Gets the BLE scan filter parameter (UUID variant) (cmd 2).
    #[inline]
    pub fn get_ble_scan_filter_parameter2(
        &self,
        parameter_id: u16,
    ) -> Result<BtdrvGattAttributeUuid, nx_sf::service::DispatchError> {
        cmif::get_ble_scan_filter_parameter2(&self.0, parameter_id)
    }

    /// Starts BLE scanning for general devices (cmd 3).
    #[inline]
    pub fn start_ble_scan_for_general(
        &self,
        param: &BtdrvBleAdvertisePacketParameter,
        applet_resource_user_id: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::start_ble_scan_for_general(&self.0, param, applet_resource_user_id)
    }

    /// Stops BLE scanning for general devices (cmd 4).
    #[inline]
    pub fn stop_ble_scan_for_general(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::stop_ble_scan_for_general(&self.0)
    }

    /// Gets BLE scan results for general devices (cmd 5).
    ///
    /// Writes results into the caller's buffer and returns the count written.
    #[inline]
    pub fn get_ble_scan_results_for_general(
        &self,
        results: &mut [BtdrvBleScanResult],
        applet_resource_user_id: u64,
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_ble_scan_results_for_general(&self.0, results, applet_resource_user_id)
    }

    /// Starts BLE scanning for paired devices (cmd 6).
    #[inline]
    pub fn start_ble_scan_for_paired(
        &self,
        param: &BtdrvBleAdvertisePacketParameter,
        applet_resource_user_id: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::start_ble_scan_for_paired(&self.0, param, applet_resource_user_id)
    }

    /// Stops BLE scanning for paired devices (cmd 7).
    #[inline]
    pub fn stop_ble_scan_for_paired(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::stop_ble_scan_for_paired(&self.0)
    }

    /// Starts BLE scanning for smart devices (cmd 8).
    #[inline]
    pub fn start_ble_scan_for_smart_device(
        &self,
        uuid: &BtdrvGattAttributeUuid,
        applet_resource_user_id: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::start_ble_scan_for_smart_device(&self.0, uuid, applet_resource_user_id)
    }

    /// Stops BLE scanning for smart devices (cmd 9).
    #[inline]
    pub fn stop_ble_scan_for_smart_device(&self) -> Result<(), nx_sf::service::DispatchError> {
        cmif::stop_ble_scan_for_smart_device(&self.0)
    }

    /// Gets BLE scan results for smart devices (cmd 10).
    ///
    /// Writes results into the caller's buffer and returns the count written.
    #[inline]
    pub fn get_ble_scan_results_for_smart_device(
        &self,
        results: &mut [BtdrvBleScanResult],
        applet_resource_user_id: u64,
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_ble_scan_results_for_smart_device(&self.0, results, applet_resource_user_id)
    }
}

// ---------------------------------------------------------------------------
// BLE connection commands
// ---------------------------------------------------------------------------

impl BtmuService {
    /// Acquires the BLE connection event (cmd 17).
    ///
    /// Returns a copy handle for the event (autoclear=true).
    #[inline]
    pub fn acquire_ble_connection_event(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_ble_connection_event(&self.0)
    }

    /// Connects to a BLE device (cmd 18).
    #[inline]
    pub fn ble_connect(
        &self,
        addr: &BtdrvAddress,
        applet_resource_user_id: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::ble_connect(&self.0, addr, applet_resource_user_id)
    }

    /// Disconnects a BLE device (cmd 19).
    #[inline]
    pub fn ble_disconnect(
        &self,
        connection_handle: u32,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::ble_disconnect(&self.0, connection_handle)
    }

    /// Gets BLE connection state (cmd 20).
    ///
    /// Writes connection info into the caller's buffer and returns the count.
    #[inline]
    pub fn ble_get_connection_state(
        &self,
        info: &mut [BtdrvBleConnectionInfo],
        applet_resource_user_id: u64,
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::ble_get_connection_state(&self.0, info, applet_resource_user_id)
    }
}

// ---------------------------------------------------------------------------
// BLE pairing commands
// ---------------------------------------------------------------------------

impl BtmuService {
    /// Acquires the BLE pairing event (cmd 21).
    ///
    /// Returns a copy handle for the event (autoclear=true).
    #[inline]
    pub fn acquire_ble_pairing_event(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_ble_pairing_event(&self.0)
    }

    /// Pairs a BLE device (cmd 22).
    #[inline]
    pub fn ble_pair_device(
        &self,
        connection_handle: u32,
        param: &BtdrvBleAdvertisePacketParameter,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::ble_pair_device(&self.0, connection_handle, param)
    }

    /// Unpairs a BLE device by connection handle (cmd 23).
    #[inline]
    pub fn ble_unpair_device(
        &self,
        connection_handle: u32,
        param: &BtdrvBleAdvertisePacketParameter,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::ble_unpair_device(&self.0, connection_handle, param)
    }

    /// Unpairs a BLE device by address (cmd 24).
    #[inline]
    pub fn ble_unpair_device2(
        &self,
        addr: &BtdrvAddress,
        param: &BtdrvBleAdvertisePacketParameter,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::ble_unpair_device2(&self.0, addr, param)
    }

    /// Gets paired BLE devices for a given advertise parameter (cmd 25).
    ///
    /// Writes device addresses into the caller's buffer and returns the count.
    #[inline]
    pub fn ble_get_paired_devices(
        &self,
        param: &BtdrvBleAdvertisePacketParameter,
        addrs: &mut [BtdrvAddress],
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::ble_get_paired_devices(&self.0, param, addrs)
    }
}

// ---------------------------------------------------------------------------
// GATT service discovery commands
// ---------------------------------------------------------------------------

impl BtmuService {
    /// Acquires the BLE service discovery event (cmd 26).
    ///
    /// Returns a copy handle for the event (autoclear=true).
    #[inline]
    pub fn acquire_ble_service_discovery_event(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_ble_service_discovery_event(&self.0)
    }

    /// Gets all GATT services for a connection (cmd 27).
    ///
    /// Writes services into the caller's buffer and returns the count.
    #[inline]
    pub fn get_gatt_services(
        &self,
        connection_handle: u32,
        services: &mut [BtmGattService],
        applet_resource_user_id: u64,
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_gatt_services(
            &self.0,
            connection_handle,
            services,
            applet_resource_user_id,
        )
    }

    /// Gets a single GATT service matching a UUID (cmd 28).
    ///
    /// Returns whether a matching service was found.
    #[inline]
    pub fn get_gatt_service(
        &self,
        connection_handle: u32,
        uuid: &BtdrvGattAttributeUuid,
        out_service: &mut BtmGattService,
        applet_resource_user_id: u64,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::get_gatt_service(
            &self.0,
            connection_handle,
            uuid,
            out_service,
            applet_resource_user_id,
        )
    }

    /// Gets included GATT services for a service handle (cmd 29).
    ///
    /// Writes services into the caller's buffer and returns the count.
    #[inline]
    pub fn get_gatt_included_services(
        &self,
        connection_handle: u32,
        service_handle: u16,
        services: &mut [BtmGattService],
        applet_resource_user_id: u64,
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_gatt_included_services(
            &self.0,
            connection_handle,
            service_handle,
            services,
            applet_resource_user_id,
        )
    }

    /// Gets the GATT service that an attribute belongs to (cmd 30).
    ///
    /// Returns whether a matching service was found.
    #[inline]
    pub fn get_belonging_gatt_service(
        &self,
        connection_handle: u32,
        attribute_handle: u16,
        out_service: &mut BtmGattService,
        applet_resource_user_id: u64,
    ) -> Result<bool, nx_sf::service::DispatchError> {
        cmif::get_belonging_gatt_service(
            &self.0,
            connection_handle,
            attribute_handle,
            out_service,
            applet_resource_user_id,
        )
    }

    /// Gets GATT characteristics for a service handle (cmd 31).
    ///
    /// Writes characteristics into the caller's buffer and returns the count.
    #[inline]
    pub fn get_gatt_characteristics(
        &self,
        connection_handle: u32,
        service_handle: u16,
        characteristics: &mut [BtmGattCharacteristic],
        applet_resource_user_id: u64,
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_gatt_characteristics(
            &self.0,
            connection_handle,
            service_handle,
            characteristics,
            applet_resource_user_id,
        )
    }

    /// Gets GATT descriptors for a characteristic handle (cmd 32).
    ///
    /// Writes descriptors into the caller's buffer and returns the count.
    #[inline]
    pub fn get_gatt_descriptors(
        &self,
        connection_handle: u32,
        char_handle: u16,
        descriptors: &mut [BtmGattDescriptor],
        applet_resource_user_id: u64,
    ) -> Result<u8, nx_sf::service::DispatchError> {
        cmif::get_gatt_descriptors(
            &self.0,
            connection_handle,
            char_handle,
            descriptors,
            applet_resource_user_id,
        )
    }
}

// ---------------------------------------------------------------------------
// BLE MTU commands
// ---------------------------------------------------------------------------

impl BtmuService {
    /// Acquires the BLE MTU configuration event (cmd 33).
    ///
    /// Returns a copy handle for the event (autoclear=true).
    #[inline]
    pub fn acquire_ble_mtu_config_event(&self) -> Result<u32, AcquireEventWithFlagError> {
        cmif::acquire_ble_mtu_config_event(&self.0)
    }

    /// Configures the BLE MTU for a connection (cmd 34).
    #[inline]
    pub fn configure_ble_mtu(
        &self,
        connection_handle: u32,
        mtu: u16,
        applet_resource_user_id: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::configure_ble_mtu(&self.0, connection_handle, mtu, applet_resource_user_id)
    }

    /// Gets the BLE MTU for a connection (cmd 35).
    #[inline]
    pub fn get_ble_mtu(
        &self,
        connection_handle: u32,
        applet_resource_user_id: u64,
    ) -> Result<u16, nx_sf::service::DispatchError> {
        cmif::get_ble_mtu(&self.0, connection_handle, applet_resource_user_id)
    }
}

// ---------------------------------------------------------------------------
// GATT data path commands
// ---------------------------------------------------------------------------

impl BtmuService {
    /// Registers a BLE GATT data path (cmd 36).
    #[inline]
    pub fn register_ble_gatt_data_path(
        &self,
        path: &BtmBleDataPath,
        applet_resource_user_id: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::register_ble_gatt_data_path(&self.0, path, applet_resource_user_id)
    }

    /// Unregisters a BLE GATT data path (cmd 37).
    #[inline]
    pub fn unregister_ble_gatt_data_path(
        &self,
        path: &BtmBleDataPath,
        applet_resource_user_id: u64,
    ) -> Result<(), nx_sf::service::DispatchError> {
        cmif::unregister_ble_gatt_data_path(&self.0, path, applet_resource_user_id)
    }
}

// ---------------------------------------------------------------------------
// Connection
// ---------------------------------------------------------------------------

/// Connects to the Bluetooth Manager User service (`btm:u`) using CMIF.
///
/// Obtains the root `btm:u` session, then extracts the IBtmUserCore
/// sub-object (cmd 0). The root session is closed automatically on `Drop`.
pub fn connect_cmif(sm: &SmService) -> Result<BtmuService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    let root = Session::new(handle, 0);

    let core_raw = cmif::get_core(&root).map_err(ConnectCmifError::GetCore)?;

    // SAFETY: the kernel returned a valid move handle for the new IBtmUserCore
    // sub-object; ownership transfers to the new `Session`.
    let core_handle = Handle::from_raw_unchecked(core_raw);

    Ok(BtmuService(Session::new(
        OwnedSessionHandle::from_handle_unchecked(core_handle),
        0,
    )))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    #[error("failed to get btm:u service")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    #[error("failed to get IBtmUserCore sub-object")]
    GetCore(#[source] cmif::GetCoreError),
}
